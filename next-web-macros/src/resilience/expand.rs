//! Code generation shared by the five attribute macros.
//!
//! Every macro does the same three things:
//!
//! 1. parse and check the attribute, reporting a `compile_error!` rather than
//!    panicking when something is wrong;
//! 2. bind the pattern instance the attribute names;
//! 3. replace the **body** of the function, leaving the signature — and, just as
//!    importantly, the other attributes on the item, so that macros can be
//!    stacked — untouched.
//!
//! # Why the body is wrapped
//!
//! A body that uses `?` or `return` leaves the *function*, not the block it sits
//! in. For the patterns that record an outcome after the guarded call, the body
//! is therefore moved into a `move` closure (synchronous) or a `move` block
//! (asynchronous), so that `?` unwinds only as far as the guarded unit and the
//! outcome is still recorded. The two patterns that only gate the call have
//! nothing to record afterwards and splice the body in unchanged.
//!
//! # Why the instance is bound to a reference
//!
//! `&(expression)` accepts every shape a caller is likely to have — `Arc<T>`,
//! `&Arc<T>`, `&T` — because method calls deref through all of them, and it
//! keeps a temporary alive for the whole call.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Block, Expr, FnArg, Ident, ItemFn, Pat, ReturnType, Signature, Type, WherePredicate};

use super::attr::{ResilienceAttr, Source};

/// Which pattern an attribute macro stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pattern {
    /// Caps how often the guarded function may be called.
    RateLimiter,
    /// Refuses calls while the dependency is unhealthy, and records the outcome.
    CircuitBreaker,
    /// Caps how many calls run at the same time.
    Bulkhead,
    /// Caps how long the guarded function may take.
    TimeLimiter,
    /// Repeats the guarded function while it fails.
    Retry,
}

impl Pattern {
    /// The attribute name, for diagnostics.
    pub(crate) fn keyword(self) -> &'static str {
        match self {
            Self::RateLimiter => "rate_limiter",
            Self::CircuitBreaker => "circuit_breaker",
            Self::Bulkhead => "bulkhead",
            Self::TimeLimiter => "time_limiter",
            Self::Retry => "retry",
        }
    }

    /// Whether the pattern can guard a synchronous function.
    ///
    /// The three that cannot need either an `await` to wait for something or a
    /// future to drop, neither of which exists in a synchronous body.
    fn supports_sync(self) -> bool {
        matches!(self, Self::CircuitBreaker | Self::Retry)
    }

    /// Why a synchronous function cannot be guarded, in the words the compiler
    /// will show the caller.
    fn sync_unsupported_reason(self) -> &'static str {
        match self {
            Self::RateLimiter => {
                "It asks its permission store for a permission, and that store may live in \
                 another process, so acquiring one has to be awaited. Awaiting inside a \
                 synchronous function would mean blocking a thread on a runtime."
            }
            Self::Bulkhead => {
                "It waits for a permit from a `tokio::sync::Semaphore`, which has to be awaited. \
                 Awaiting inside a synchronous function would mean blocking a thread on a \
                 runtime."
            }
            Self::TimeLimiter => {
                "It enforces the limit by dropping the guarded future, and a synchronous call has \
                 no future to drop, so there would be no way to abandon it."
            }
            Self::CircuitBreaker | Self::Retry => "",
        }
    }

    /// The accessor the process registry exposes for this pattern.
    fn registry_accessor(self) -> &'static str {
        match self {
            Self::RateLimiter => "rate_limiter_or_default",
            Self::CircuitBreaker => "circuit_breaker_or_default",
            Self::Bulkhead => "bulkhead_or_default",
            Self::TimeLimiter => "time_limiter_or_default",
            Self::Retry => "retry_or_default",
        }
    }
}

/// Expands one attribute macro over one function.
pub(crate) fn apply(pattern: Pattern, attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = match syn::parse2::<ResilienceAttr>(attr) {
        Ok(attr) => attr,
        Err(error) => return error.to_compile_error(),
    };
    let mut func = match syn::parse2::<ItemFn>(item) {
        Ok(func) => func,
        Err(error) => return error.to_compile_error(),
    };

    if let Err(error) = check(pattern, &func) {
        return error.to_compile_error();
    }

    let is_async = func.sig.asyncness.is_some();
    let body = (*func.block).clone();
    let instance = bind_instance(pattern, &attr);
    let fallback = attr.fallback.as_ref();
    // Every pattern but the retry reports a rejection under this name; the retry
    // loop builds its own, because it degrades only once the budget is spent.
    let rejection = quote!(__nw_rejection);

    let statements = match pattern {
        Pattern::RateLimiter => {
            let reject = reject(fallback, &rejection);
            quote! {
                let __nw_pattern = #instance;
                let __nw_timeout = __nw_pattern.config().timeout_duration();
                if let ::core::result::Result::Err(#rejection) =
                    __nw_pattern.acquire_permission_with_timeout(__nw_timeout).await
                {
                    #reject
                }
                #body
            }
        }
        Pattern::Bulkhead => {
            let reject = reject(fallback, &rejection);
            quote! {
                let __nw_pattern = #instance;
                // `_nw_permit` rather than `_`, which would release the slot at the
                // end of this statement instead of at the end of the call.
                let _nw_permit = match __nw_pattern.acquire_permission().await {
                    ::core::result::Result::Ok(__nw_permit) => __nw_permit,
                    ::core::result::Result::Err(#rejection) => {
                        #reject
                    }
                };
                #body
            }
        }
        Pattern::CircuitBreaker => {
            let reject = reject(fallback, &rejection);
            let unit = timed_unit(&body, is_async);
            quote! {
                let __nw_pattern = #instance;
                if let ::core::result::Result::Err(#rejection) = __nw_pattern.acquire_permission()
                {
                    #reject
                }
                let __nw_started = ::std::time::Instant::now();
                let __nw_outcome = #unit;
                let __nw_elapsed = __nw_started.elapsed();
                match &__nw_outcome {
                    ::core::result::Result::Ok(_) => __nw_pattern.on_success(__nw_elapsed),
                    ::core::result::Result::Err(__nw_error) => {
                        __nw_pattern.on_error(__nw_elapsed, __nw_error)
                    }
                }
                __nw_outcome
            }
        }
        Pattern::TimeLimiter => {
            let reject = reject(fallback, &rejection);
            // Only the time limiter has to go through the pattern's own `call`:
            // it is the only way to enforce the limit and record the timeout.
            let error = quote!(::next_web_resilience::timelimiter::TimeLimiterError);
            quote! {
                let __nw_pattern = #instance;
                let __nw_outcome = __nw_pattern.call(move || async move { #body }).await;
                match __nw_outcome {
                    ::core::result::Result::Ok(__nw_value) => {
                        ::core::result::Result::Ok(__nw_value)
                    }
                    ::core::result::Result::Err(#error::Inner(__nw_error)) => {
                        ::core::result::Result::Err(__nw_error)
                    }
                    ::core::result::Result::Err(#error::Timeout(#rejection)) => {
                        #reject
                    }
                    ::core::result::Result::Err(#error::Aborted(#rejection)) => {
                        #reject
                    }
                }
            }
        }
        Pattern::Retry => retry_statements(&instance, &body, fallback, is_async, &mut func.sig),
    };

    func.block = match syn::parse2::<Block>(quote!({ #statements })) {
        Ok(block) => Box::new(block),
        Err(error) => return error.to_compile_error(),
    };

    // println!("token stream: {}", func.to_token_stream().to_string());

    quote!(#func)
}

/// The statements that repeat the guarded body until it succeeds or gives up.
///
/// The loop drives the retry through its public primitives rather than through
/// `Retry::call`, because every attempt needs the parameters again and a closure
/// cannot return a future that borrows its own captures. Driving the primitives
/// directly leaves the retry's counters and event stream exactly as `call` would.
fn retry_statements(
    instance: &TokenStream,
    body: &Block,
    fallback: Option<&Expr>,
    is_async: bool,
    sig: &mut Signature,
) -> TokenStream {
    let (clones, bounds) = retry_parameter_clones(sig);
    extend_where_clause(sig, bounds);

    let unit = timed_unit(body, is_async);
    let wait = if is_async {
        quote! { ::tokio::time::sleep(__nw_wait).await; }
    } else {
        quote! { ::std::thread::sleep(__nw_wait); }
    };
    // A spent budget degrades to the fallback when one is given, and otherwise
    // reports the failure of the last attempt.
    let exhausted = if let Some(expression) = fallback {
        quote! { return #expression; }
    } else {
        quote! { return ::core::result::Result::Err(__nw_error); }
    };

    quote! {
        let __nw_pattern = #instance;
        let __nw_max_attempts = __nw_pattern.config().max_attempts();
        let mut __nw_attempt: u32 = 0;
        loop {
            __nw_attempt += 1;
            #(#clones)*
            let __nw_started = ::std::time::Instant::now();
            let __nw_outcome = #unit;
            let __nw_elapsed = __nw_started.elapsed();
            match __nw_outcome {
                ::core::result::Result::Ok(__nw_value) => {
                    __nw_pattern.record_success(__nw_attempt, __nw_elapsed);
                    return ::core::result::Result::Ok(__nw_value);
                }
                ::core::result::Result::Err(__nw_error) => {
                    if !__nw_pattern.should_retry(&__nw_error) {
                        // The policy refuses this failure on purpose, so it goes
                        // to the caller rather than to the fallback.
                        __nw_pattern.record_ignored_error(
                            __nw_attempt,
                            ::std::string::ToString::to_string(&__nw_error),
                        );
                        return ::core::result::Result::Err(__nw_error);
                    }
                    if __nw_attempt >= __nw_max_attempts {
                        __nw_pattern.record_failure(
                            __nw_attempt,
                            ::core::option::Option::Some(
                                ::std::string::ToString::to_string(&__nw_error),
                            ),
                        );
                        #exhausted
                    }
                    let __nw_wait = __nw_pattern.prepare_next_attempt(
                        __nw_attempt,
                        ::std::string::ToString::to_string(&__nw_error),
                    );
                    #wait
                }
            }
        }
    }
}

/// The tokens that run the guarded body once and produce its outcome.
///
/// Wrapping the body keeps a `?` or a `return` inside it from leaving the
/// function before the outcome has been recorded.
fn timed_unit(body: &Block, is_async: bool) -> TokenStream {
    if is_async {
        quote! { async move { #body }.await }
    } else {
        quote! { (move || #body)() }
    }
}

/// The tokens that leave the function when a rejection is produced.
fn reject(fallback: Option<&Expr>, rejection: &TokenStream) -> TokenStream {
    if let Some(expression) = fallback {
        quote! { return #expression; }
    } else {
        quote! {
            return ::core::result::Result::Err(::core::convert::Into::into(#rejection));
        }
    }
}

/// The tokens that bind the pattern instance.
fn bind_instance(pattern: Pattern, attr: &ResilienceAttr) -> TokenStream {
    match &attr.source {
        Source::Instance(expression) => quote! { &(#expression) },
        Source::Name(name) => {
            let accessor = Ident::new(pattern.registry_accessor(), Span::call_site());
            quote! { ::next_web_resilience::global_registry().#accessor(#name) }
        }
    }
}

/// The statements that re-create the parameters for one retry attempt, and the
/// `Clone` bounds to add for parameters of a generic type.
///
/// A parameter taken by reference is borrowed for the whole call and needs no
/// copy; every other parameter is cloned so the body can consume it again.
fn retry_parameter_clones(sig: &Signature) -> (Vec<TokenStream>, Vec<WherePredicate>) {
    let type_parameters: Vec<String> = sig
        .generics
        .type_params()
        .map(|parameter| parameter.ident.to_string())
        .collect();

    let mut clones = Vec::new();
    let mut bounds = Vec::new();

    for input in &sig.inputs {
        let FnArg::Typed(pat_type) = input else {
            // A receiver is a reference; `self` cannot be rebound, so a
            // by-value receiver is rejected before this point.
            continue;
        };
        if matches!(&*pat_type.ty, Type::Reference(_)) {
            continue;
        }
        let Pat::Ident(pat_ident) = &*pat_type.pat else {
            // A destructuring parameter has no single name to rebind.
            continue;
        };
        if pat_ident.by_ref.is_some() {
            continue;
        }

        let ident = &pat_ident.ident;
        let mutability = &pat_ident.mutability;
        clones.push(quote! {
            let #mutability #ident = ::core::clone::Clone::clone(&#ident);
        });

        // Naming the bound on the function is far friendlier than letting the
        // clone above fail with "method `clone` not found".
        if let Type::Path(path) = &*pat_type.ty {
            let is_bare_type_parameter = path.qself.is_none()
                && path.path.segments.len() == 1
                && type_parameters.contains(&path.path.segments[0].ident.to_string());
            if is_bare_type_parameter {
                let ty = &pat_type.ty;
                bounds.push(syn::parse_quote!(#ty: ::core::clone::Clone));
            }
        }
    }

    (clones, bounds)
}

/// Adds `bounds` to the function's `where` clause, creating one if needed.
fn extend_where_clause(sig: &mut Signature, bounds: Vec<WherePredicate>) {
    if bounds.is_empty() {
        return;
    }

    sig.generics
        .make_where_clause()
        .predicates
        .extend(bounds);
}

/// Rejects the attributes that cannot work, with a message that says why.
fn check(pattern: Pattern, func: &ItemFn) -> syn::Result<()> {
    if let Some(constness) = func.sig.constness {
        return Err(syn::Error::new_spanned(
            constness,
            format!(
                "`#[{}]` cannot guard a `const fn`: the guard keeps state between calls, which \
                 constant evaluation cannot do",
                pattern.keyword()
            ),
        ));
    }

    if func.sig.asyncness.is_none() && !pattern.supports_sync() {
        return Err(syn::Error::new_spanned(
            &func.sig.ident,
            format!(
                "`#[{}]` cannot guard a synchronous function. {} Write the guarded function as \
                 `async fn`, or drive the pattern by hand inside the body.",
                pattern.keyword(),
                pattern.sync_unsupported_reason()
            ),
        ));
    }

    if returns_boxed_future(func) {
        return Err(syn::Error::new_spanned(
            &func.sig.output,
            format!(
                "`#[{}]` found a method that returns a boxed future rather than being an \
                 `async fn`. That is what `#[async_trait]` produces: it is applied to the whole \
                 impl block, so it expands before any attribute on the methods inside it, and by \
                 the time this macro runs there is no `async fn` left to guard. Drop \
                 `#[async_trait]` — an `async fn` in a trait is native — or leave this attribute \
                 off and drive the pattern from inside the body.",
                pattern.keyword()
            ),
        ));
    }

    if !returns_result(func) {
        return Err(syn::Error::new_spanned(
            &func.sig.output,
            format!(
                "`#[{}]` needs the guarded function to return `Result<_, _>` so that a rejection \
                 can be handed to the caller. A return type written as a type alias is not \
                 recognised: spell out `Result`.",
                pattern.keyword()
            ),
        ));
    }

    // This was a let-chain — `if <cond> && let Some(..) = ..` — which edition
    // 2021 does not accept, so the two conditions are nested instead. The
    // behaviour is the same: the retry is refused exactly when the first input
    // is a receiver that is not taken by reference.
    if pattern == Pattern::Retry {
        if let Some(FnArg::Receiver(receiver)) = func.sig.inputs.first() {
            if receiver.reference.is_none() {
                return Err(syn::Error::new_spanned(
                    receiver,
                    "`#[retry]` cannot guard a method that takes `self` by value, because every \
                     attempt would need the receiver again and `self` cannot be rebound. Take \
                     `&self` or `&mut self`, or move the retried work into such a method.",
                ));
            }
        }
    }

    Ok(())
}

/// Returns whether the function returns `Result<_, _>` written out.
fn returns_result(func: &ItemFn) -> bool {
    let ReturnType::Type(_, ty) = &func.sig.output else {
        return false;
    };
    let Type::Path(path) = &**ty else {
        return false;
    };
    path.path.segments.last().is_some_and(|segment| {
        let name = segment.ident.to_string();
        name == "Result" || name == "StdResult"
    })
}

/// Returns whether the return type is a boxed future, which is the shape
/// `#[async_trait]` leaves behind.
///
/// Matched on the rendered tokens rather than walked: the check only exists to
/// choose a better diagnostic, and the surrounding tests pin the message down.
fn returns_boxed_future(func: &ItemFn) -> bool {
    let ReturnType::Type(_, ty) = &func.sig.output else {
        return false;
    };
    let rendered = quote!(#ty).to_string();
    rendered.contains("Pin") && rendered.contains("Future")
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::{ItemFn, ReturnType, Type, Visibility};

    use super::{Pattern, apply};

    /// Renders an expansion as source text with every space removed.
    ///
    /// `TokenStream`'s own rendering puts spaces in places that depend on how
    /// neighbouring punctuation is spaced — an empty closure comes out as
    /// `move | |` — so a token sequence only compares stably once the whitespace
    /// is gone.
    fn render(
        pattern: Pattern,
        attr: proc_macro2::TokenStream,
        item: proc_macro2::TokenStream,
    ) -> String {
        apply(pattern, attr, item).to_string().split_whitespace().collect()
    }

    /// Renders an expansion as source text, spaces intact.
    ///
    /// Needed to assert on a message inside a string literal, whose own spaces
    /// are part of what is being checked.
    fn render_text(
        pattern: Pattern,
        attr: proc_macro2::TokenStream,
        item: proc_macro2::TokenStream,
    ) -> String {
        apply(pattern, attr, item).to_string()
    }

    /// Parses an expansion back into a function, so the assertions describe the
    /// syntax tree rather than the spacing `to_string` happens to produce.
    fn expanded_function(
        pattern: Pattern,
        attr: proc_macro2::TokenStream,
        item: proc_macro2::TokenStream,
    ) -> ItemFn {
        syn::parse2(apply(pattern, attr, item)).expect("the expansion should be a function")
    }

    /// A function with the given signature and a trivial body.
    fn function(
        signature: proc_macro2::TokenStream,
        body: proc_macro2::TokenStream,
    ) -> proc_macro2::TokenStream {
        quote! { #signature { #body } }
    }

    /// The accessor chain of the expansion, when it used the registry form.
    fn was_compiled(pattern: Pattern, attr: proc_macro2::TokenStream) -> bool {
        render(
            pattern,
            attr,
            function(quote!(async fn charge(&self) -> Result<u64, Error>), quote!(Ok(1))),
        )
        .contains("compile_error")
    }

    #[test]
    fn the_signature_and_the_stacked_attributes_survive() {
        let func = expanded_function(
            Pattern::CircuitBreaker,
            quote!(self.breaker),
            quote! {
                #[inline]
                pub(crate) async fn charge(&self, id: u64) -> Result<u64, Error> {
                    Ok(id)
                }
            },
        );

        // The attribute stacked underneath must still be on the item, otherwise
        // the next macro in the chain would never run.
        assert_eq!(func.attrs.len(), 1);
        assert!(matches!(func.vis, Visibility::Restricted(_)));
        assert!(func.sig.asyncness.is_some());
        assert_eq!(func.sig.ident, "charge");
        assert_eq!(func.sig.inputs.len(), 2);
        match &func.sig.output {
            ReturnType::Type(_, ty) => {
                let Type::Path(path) = &**ty else {
                    panic!("expected a path return type");
                };
                assert_eq!(path.path.segments.last().expect("a segment").ident, "Result");
            }
            ReturnType::Default => panic!("the return type was dropped"),
        }
    }

    #[test]
    fn a_generic_signature_keeps_its_parameters() {
        let func = expanded_function(
            Pattern::CircuitBreaker,
            quote!(self.breaker),
            function(
                quote!(pub async fn charge<T: Send>(&self, value: T) -> Result<T, Error>),
                quote!(Ok(value)),
            ),
        );

        assert_eq!(func.sig.generics.type_params().count(), 1);
    }

    #[test]
    fn the_circuit_breaker_records_both_outcomes() {
        let rendered = render(
            Pattern::CircuitBreaker,
            quote!(self.breaker),
            function(
                quote!(async fn charge(&self) -> Result<u64, Error>),
                quote!(Ok(1)),
            ),
        );

        assert!(rendered.contains("acquire_permission"), "{rendered}");
        assert!(rendered.contains("on_success"), "{rendered}");
        assert!(rendered.contains("on_error"), "{rendered}");
        assert!(rendered.contains("Instant"), "{rendered}");
    }

    #[test]
    fn the_guarded_body_is_wrapped_so_that_a_question_mark_cannot_escape() {
        let rendered = render(
            Pattern::CircuitBreaker,
            quote!(self.breaker),
            function(
                quote!(async fn charge(&self) -> Result<u64, Error>),
                quote!(let value = fallible()?; Ok(value)),
            ),
        );

        assert!(rendered.contains("asyncmove"), "{rendered}");
    }

    #[test]
    fn a_synchronous_body_is_wrapped_in_a_closure() {
        let rendered = render(
            Pattern::CircuitBreaker,
            quote!(self.breaker),
            function(
                quote!(fn charge(&self) -> Result<u64, Error>),
                quote!(let value = fallible()?; Ok(value)),
            ),
        );

        assert!(rendered.contains("move||"), "{rendered}");
    }

    #[test]
    fn a_rejection_without_a_fallback_is_converted() {
        let rendered = render(
            Pattern::CircuitBreaker,
            quote!(self.breaker),
            function(
                quote!(async fn charge(&self) -> Result<u64, Error>),
                quote!(Ok(1)),
            ),
        );

        assert!(rendered.contains("Into"), "{rendered}");
    }

    #[test]
    fn a_fallback_replaces_the_conversion() {
        let rendered = render(
            Pattern::CircuitBreaker,
            quote!(self.breaker, fallback = Ok(0)),
            function(
                quote!(async fn charge(&self) -> Result<u64, Error>),
                quote!(Ok(1)),
            ),
        );

        assert!(!rendered.contains("Into"), "{rendered}");
        assert!(rendered.contains("Ok(0)"), "{rendered}");
    }

    #[test]
    fn the_bulkhead_keeps_the_permit_binding_alive() {
        let rendered = render(
            Pattern::Bulkhead,
            quote!(self.bulkhead),
            function(
                quote!(async fn charge(&self) -> Result<u64, Error>),
                quote!(Ok(1)),
            ),
        );

        // `_nw_permit` rather than `_`: a bare underscore would drop the permit
        // immediately and the bulkhead would bound nothing.
        assert!(rendered.contains("_nw_permit"), "{rendered}");
        assert!(rendered.contains("acquire_permission"), "{rendered}");
    }

    #[test]
    fn the_time_limiter_goes_through_call_so_that_the_timeout_is_recorded() {
        let rendered = render(
            Pattern::TimeLimiter,
            quote!(self.limiter),
            function(
                quote!(async fn charge(&self) -> Result<u64, Error>),
                quote!(Ok(1)),
            ),
        );

        assert!(rendered.contains("TimeLimiterError"), "{rendered}");
        assert!(rendered.contains("Timeout"), "{rendered}");
        assert!(rendered.contains("Aborted"), "{rendered}");
    }

    #[test]
    fn the_retry_clones_its_owned_parameters() {
        let rendered = render(
            Pattern::Retry,
            quote!(self.retry),
            function(
                quote!(async fn charge(&self, id: String) -> Result<u64, Error>),
                quote!(Ok(1)),
            ),
        );

        assert!(rendered.contains("Clone"), "{rendered}");
        assert!(rendered.contains("should_retry"), "{rendered}");
        assert!(rendered.contains("prepare_next_attempt"), "{rendered}");
        assert!(rendered.contains("record_success"), "{rendered}");
        assert!(rendered.contains("record_failure"), "{rendered}");
        assert!(rendered.contains("record_ignored_error"), "{rendered}");
    }

    #[test]
    fn the_retry_does_not_clone_borrowed_parameters() {
        let func = expanded_function(
            Pattern::Retry,
            quote!(self.retry),
            function(
                quote!(async fn charge(&self, id: &str) -> Result<u64, Error>),
                quote!(Ok(1)),
            ),
        );

        // Only the `&self` receiver and the `&str` parameter, so nothing to
        // clone and no `Clone` bound to add.
        assert!(func.sig.generics.where_clause.is_none());
    }

    #[test]
    fn the_retry_names_the_clone_bound_for_a_type_parameter() {
        let func = expanded_function(
            Pattern::Retry,
            quote!(self.retry),
            function(
                quote!(async fn charge<T>(&self, value: T) -> Result<u64, Error>),
                quote!(Ok(1)),
            ),
        );

        let where_clause = func
            .sig
            .generics
            .where_clause
            .expect("a `Clone` bound should have been added");
        assert_eq!(where_clause.predicates.len(), 1);
    }

    #[test]
    fn a_name_resolves_through_the_process_registry() {
        let rendered = render(
            Pattern::RateLimiter,
            quote!(name = "login"),
            function(
                quote!(async fn charge(&self) -> Result<u64, Error>),
                quote!(Ok(1)),
            ),
        );

        assert!(rendered.contains("global_registry"), "{rendered}");
        assert!(rendered.contains("rate_limiter_or_default"), "{rendered}");
        assert!(rendered.contains("\"login\""), "{rendered}");
    }

    #[test]
    fn an_expression_is_bound_by_reference() {
        let rendered = render(
            Pattern::RateLimiter,
            quote!(self.limiters.login),
            function(
                quote!(async fn charge(&self) -> Result<u64, Error>),
                quote!(Ok(1)),
            ),
        );

        assert!(rendered.contains("&(self.limiters.login)"), "{rendered}");
        assert!(!rendered.contains("global_registry"), "{rendered}");
    }

    #[test]
    fn a_synchronous_function_is_refused_for_an_async_only_pattern() {
        for pattern in [Pattern::RateLimiter, Pattern::Bulkhead, Pattern::TimeLimiter] {
            // Rebuilt for each rendering, because an expansion consumes its
            // input.
            let item = || {
                function(
                    quote!(fn charge(&self) -> Result<u64, Error>),
                    quote!(Ok(1)),
                )
            };
            let attr = || quote!(self.pattern);

            let rendered = render(pattern, attr(), item());
            let text = render_text(pattern, attr(), item());

            assert!(
                rendered.contains("compile_error"),
                "{}: {text}",
                pattern.keyword()
            );
            assert!(
                text.contains("cannot guard a synchronous function"),
                "{}: {text}",
                pattern.keyword()
            );
        }
    }

    #[test]
    fn a_synchronous_function_is_allowed_for_the_two_that_can() {
        for pattern in [Pattern::CircuitBreaker, Pattern::Retry] {
            assert!(
                !was_compiled(pattern, quote!(self.pattern)),
                "{} should accept a synchronous function",
                pattern.keyword()
            );
        }
    }

    #[test]
    fn a_body_that_is_not_a_result_is_refused() {
        let rendered = render(
            Pattern::CircuitBreaker,
            quote!(self.breaker),
            function(quote!(async fn charge(&self) -> u64), quote!(1)),
        );

        assert!(rendered.contains("compile_error"), "{rendered}");
        assert!(rendered.contains("Result"), "{rendered}");
    }

    #[test]
    fn a_method_left_behind_by_async_trait_is_diagnosed_by_name() {
        // What `#[async_trait]` leaves behind: a plain `fn` returning a boxed
        // future, with the attribute still attached.
        let item = || {
            quote! {
                fn charge<'life0, 'async_trait>(
                    &'life0 self,
                ) -> ::core::pin::Pin<
                    Box<
                        dyn ::core::future::Future<Output = Result<u64, Error>>
                            + ::core::marker::Send
                            + 'async_trait,
                    >,
                > {
                    Box::pin(async move { Ok(1) })
                }
            }
        };
        let attr = || quote!(self.breaker);

        let rendered = render(Pattern::CircuitBreaker, attr(), item());
        let text = render_text(Pattern::CircuitBreaker, attr(), item());

        assert!(rendered.contains("compile_error"), "{text}");
        assert!(text.contains("async_trait"), "{text}");
        assert!(text.contains("boxed future"), "{text}");
    }

    #[test]
    fn a_const_function_is_refused() {
        let rendered = render(
            Pattern::CircuitBreaker,
            quote!(self.breaker),
            function(
                quote!(const fn charge(&self) -> Result<u64, Error>),
                quote!(Ok(1)),
            ),
        );

        assert!(rendered.contains("compile_error"), "{rendered}");
        assert!(rendered.contains("constfn"), "{rendered}");
    }

    #[test]
    fn a_by_value_receiver_is_refused_for_the_retry() {
        let item = || {
            function(
                quote!(async fn charge(self) -> Result<u64, Error>),
                quote!(Ok(1)),
            )
        };
        let attr = || quote!(self.retry);

        let rendered = render(Pattern::Retry, attr(), item());
        let text = render_text(Pattern::Retry, attr(), item());

        assert!(rendered.contains("compile_error"), "{text}");
        assert!(text.contains("cannot be rebound"), "{text}");
    }

    #[test]
    fn a_by_value_receiver_is_allowed_for_the_single_call_patterns() {
        let rendered = render(
            Pattern::CircuitBreaker,
            quote!(self.breaker),
            function(
                quote!(async fn charge(self) -> Result<u64, Error>),
                quote!(Ok(1)),
            ),
        );

        assert!(!rendered.contains("compile_error"), "{rendered}");
    }

    #[test]
    fn an_empty_attribute_is_refused() {
        let rendered = render(
            Pattern::Bulkhead,
            quote!(),
            function(
                quote!(async fn charge(&self) -> Result<u64, Error>),
                quote!(Ok(1)),
            ),
        );

        assert!(rendered.contains("compile_error"), "{rendered}");
    }
}
