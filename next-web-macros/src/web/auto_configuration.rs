//! The `#[configuration]` and `#[auto_configuration]` attributes.
//!
//! Both attributes turn an `impl` block into a *configuration class*: every
//! method of the block that declares a provider is called while the application
//! starts, resolves the parameters of the method from the application context,
//! and registers the value the method returns as a singleton of the context.
//!
//! The two attributes differ in where the class is applied:
//!
//! - `#[configuration]` declares the configuration of an application. The class
//!   implements [`Configuration`](::next_web_core::config::Configuration)
//!   and is applied before every auto-configuration. Its methods may also
//!   declare an `#[init]` initializer, which runs without registering the value
//!   it computes.
//! - `#[auto_configuration]` declares the auto-configuration of a starter. The
//!   class implements
//!   [`AutoConfiguration`](::next_web_core::config::auto_configuration::AutoConfiguration)
//!   and is applied together with the auto-configurations of the application.
//!
//! The configurations a configuration has to be applied before or after are
//! declared with the `before` and `after` attributes, which name the
//! configurations after their types.

use std::collections::HashSet;

use from_attr::FromAttr;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{parse2, parse_quote, Stmt};
use syn::{
    Error, Expr, ExprPath, FnArg, Ident, ImplItem, ImplItemFn, ItemImpl, LitStr, Pat, PatType,
    ReturnType, Signature, Type,
};

use crate::{
    di::commons::{
        check_reference_borrows, push_resolve_stmts, BorrowedReference, ResolveOneValue,
    },
    util::logic::Logic,
    web::attrs::auto_configuration_attr::*,
};

/// The order a step runs in when the configuration does not declare one, which
/// is the default order of [`Ordered`](::next_web_core::Ordered).
const DEFAULT_ORDER: &str = "100";

/// The kind of configuration class an attribute macro expands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConfigurationKind {
    /// A `#[configuration]` class of an application.
    Configuration,
    /// An `#[auto_configuration]` class of a starter.
    AutoConfiguration,
}

impl ConfigurationKind {
    /// Returns the path of the trait the generated class implements.
    fn trait_path(self) -> TokenStream2 {
        match self {
            Self::Configuration => {
                quote!(::next_web_core::config::Configuration)
            }
            Self::AutoConfiguration => {
                quote!(::next_web_core::config::AutoConfiguration)
            }
        }
    }

    /// Returns the name of the method that returns the name of the class.
    fn name_method(self) -> Ident {
        match self {
            Self::Configuration => Ident::new("configuration_name", Span::call_site()),
            Self::AutoConfiguration => Ident::new("auto_configuration_name", Span::call_site()),
        }
    }

    /// Returns the name of the method that declares the configurations this one
    /// is applied before.
    fn before_method(self) -> Ident {
        match self {
            Self::Configuration => Ident::new("configure_before", Span::call_site()),
            Self::AutoConfiguration => Ident::new("auto_configure_before", Span::call_site()),
        }
    }

    /// Returns the name of the method that declares the configurations this one
    /// is applied after.
    fn after_method(self) -> Ident {
        match self {
            Self::Configuration => Ident::new("configure_after", Span::call_site()),
            Self::AutoConfiguration => Ident::new("auto_configure_after", Span::call_site()),
        }
    }

    /// Whether the class supports `#[init]` initializers.
    fn supports_init(self) -> bool {
        matches!(self, Self::Configuration)
    }

    /// The name of the attribute macro, which the diagnostics mention.
    fn macro_name(self) -> &'static str {
        match self {
            Self::Configuration => "configuration",
            Self::AutoConfiguration => "auto_configuration",
        }
    }
}

pub fn impl_macro_auto_configuration(attrs: TokenStream, item_impl: ItemImpl) -> TokenStream {
    Logic::generate(|| generate_auto_configuration(attrs.into(), item_impl))
}

pub fn impl_macro_configuration(attrs: TokenStream, item_impl: ItemImpl) -> TokenStream {
    Logic::generate(|| generate_configuration(attrs.into(), item_impl))
}

/// Generates the configuration class of an `#[auto_configuration]` block.
pub(crate) fn generate_auto_configuration(
    attrs: TokenStream2,
    item_impl: ItemImpl,
) -> Result<TokenStream2, Error> {
    generate_configuration_class(ConfigurationKind::AutoConfiguration, attrs, item_impl)
}

/// Generates the configuration class of a `#[configuration]` block.
pub(crate) fn generate_configuration(
    attrs: TokenStream2,
    item_impl: ItemImpl,
) -> Result<TokenStream2, Error> {
    generate_configuration_class(ConfigurationKind::Configuration, attrs, item_impl)
}

/// Generates the code of the class `kind` describes for the annotated
/// implementation block.
///
/// # Errors
///
/// Returns an error when the annotated item is not an implementation block of a
/// configuration class, when one of its methods is not a valid provider or
/// initializer, or when an attribute of the macro is invalid.
///
/// # Arguments
///
/// * `kind` - The kind of configuration class to generate.
/// * `attrs` - The attributes of the macro.
/// * `item_impl` - The annotated implementation block.
fn generate_configuration_class(
    kind: ConfigurationKind,
    attrs: TokenStream2,
    mut item_impl: ItemImpl,
) -> Result<TokenStream2, Error> {
    safety_check(&item_impl, kind)?;

    let configuration = AutoConfigurationAttr::from_tokens(attrs)?;

    let type_path = match item_impl.self_ty.as_ref() {
        Type::Path(type_path) => type_path.path.segments.last(),
        _ => return Err(Error::new(item_impl.self_ty.span(), "")),
    };

    let path = match type_path {
        Some(path) => path,
        None => return Err(Error::new(type_path.span(), "")),
    };

    // The type the configuration is generated for, which the conditions of the
    // configuration name with `Self`.
    let annotated = path.ident.clone();

    // The type that carries the generated implementation. Its name is derived
    // from the annotated type, so the configurations of two types do not
    // collide.
    let registration = Ident::new(&format!("__{}Gen", annotated), Span::call_site());

    // The provider that hands the configuration out is registered under the name
    // of the generated type, which keeps it apart from the provider of the
    // annotated type itself, when that type declares one as well.
    let name = match configuration.name {
        Some(name) if name.value().trim().is_empty() => {
            return Err(Error::new(name.span(), "name must not be empty"));
        }
        Some(name) => name,
        None => LitStr::new(&registration.to_string(), registration.span()),
    };

    let order: Expr = match configuration.order {
        Some(order) => order,
        None => default_order()?,
    };

    let conditions = configuration
        .conditional
        .into_iter()
        .map(|mut condition| {
            qualify_self(&mut condition, &annotated);
            condition
        })
        .collect::<Vec<_>>();

    let (steps, condition_names) = collect_steps(&mut item_impl.items, kind, &annotated)?;

    // The functions a step names as its condition. A condition that is named
    // without a path, `conditional = [enabled]`, is a function of the module of
    // the configuration, which is emitted with the steps so that the name
    // resolves.
    let mut condition_functions = Vec::new();
    for item_fn in item_impl.items.iter().filter_map(|item| match item {
        ImplItem::Fn(item_fn) => Some(item_fn),
        _ => None,
    }) {
        if condition_names
            .iter()
            .any(|name| name == &item_fn.sig.ident.to_string())
        {
            match &item_fn.sig.output {
                ReturnType::Default => {
                    return Err(Error::new(
                        item_fn.span(),
                        "the condition function has to declare the return type bool",
                    ));
                }
                ReturnType::Type(_, ty) => match ty.as_ref() {
                    Type::Path(type_path) if type_path.path.is_ident("bool") => {
                        condition_functions.push(item_fn);
                    }
                    _ => {
                        return Err(Error::new(
                            item_fn.span(),
                            "the condition function has to return a boolean value",
                        ));
                    }
                },
            }
        }
    }

    let matches = if conditions.is_empty() {
        quote! {}
    } else {
        let conditions = conditions.iter();

        quote! {
            fn matches(&self, ctx: &dyn ::next_web_core::ApplicationContext) -> bool {
                #[allow(unused_parens)]
                if !( #(#conditions(ctx))&&* ) {
                    return false;
                }

                true
            }
        }
    };

    let before_names = configuration_names(configuration.before, "before")?;
    let after_names = configuration_names(configuration.after, "after")?;

    let before_method = kind.before_method();
    let after_method = kind.after_method();

    let before = if before_names.is_empty() {
        quote! {}
    } else {
        quote! {
            fn #before_method(&self) -> &'static [&'static str] {
                &[ #(#before_names),* ]
            }
        }
    };

    let after = if after_names.is_empty() {
        quote! {}
    } else {
        quote! {
            fn #after_method(&self) -> &'static [&'static str] {
                &[ #(#after_names),* ]
            }
        }
    };

    // The name of the configuration is the name of the annotated type and not
    // the name of the type the macro generates for it, because the name is the
    // one the `before` and `after` lists of the other configurations refer to
    // this one with.
    let configuration_name = LitStr::new(&annotated.to_string(), annotated.span());
    let name_method = kind.name_method();
    let trait_path = kind.trait_path();

    let steps_code = gen_code(steps);

    let expanded = quote! {
        #item_impl

        #[doc(hidden)]
        #[derive(Clone)]
        #[allow(non_camel_case_types)]
        struct #registration;

        #[::next_web_core::async_trait]
        impl #trait_path for #registration {
            async fn configure(
                &mut self,
                ctx: &mut dyn ::next_web_core::ApplicationContext,
            ) -> ::core::result::Result<(), ::std::boxed::Box<dyn ::std::error::Error>> {
                use ::next_web_context::ApplicationContextExt as _;

                #(#condition_functions)*

                #steps_code

                ::core::result::Result::Ok(())
            }

            #matches

            fn #name_method(&self) -> &'static str {
                #configuration_name
            }

            #before
            #after
        }

        impl ::next_web_core::Ordered for #registration {
            fn order(&self) -> i32 {
                #order
            }
        }

        // The configuration is contributed like the auto-configurations of a
        // starter, so the context applies it while it refreshes.
        ::next_web_core::register_provider!(
            <::next_web_context::Provider<
                ::std::boxed::Box<dyn #trait_path>
            > as ::core::convert::From<_>>::from(
                ::next_web_context::singleton(
                    |_: &mut dyn ::next_web_core::ApplicationContext| {
                        ::std::boxed::Box::new(#registration)
                            as ::std::boxed::Box<dyn #trait_path>
                    }
                )
                .name(#name)
            )
        );
    };

    Ok(expanded)
}

/// A step of a configuration, which is either a provider or an initializer.
struct Step {
    /// The position of the method in the implementation block, which selects the
    /// block that runs the step.
    index: usize,
    /// The expression that yields the order the step runs in.
    order: Expr,
    /// The block that runs the step.
    body: TokenStream2,
}

/// Collects the steps of a configuration and the names of the conditions its
/// steps refer to.
///
/// A method that declares a provider (`#[provider]` or
/// `#[conditional_on_property]`) becomes a step that registers the instance it
/// returns. A method that declares an initializer (`#[init]`) becomes a step
/// that runs without registering a value, which is only supported by
/// `#[configuration]`.
///
/// # Errors
///
/// Returns an error for a provider without a return type, for an initializer
/// with one, for an initializer of an `#[auto_configuration]`, and for a method
/// that declares both a provider and an initializer.
///
/// # Arguments
///
/// * `items` - The items of the annotated implementation block.
/// * `kind` - The kind of configuration class the methods belong to.
/// * `annotated` - The type the configuration is generated for.
fn collect_steps(
    items: &mut [ImplItem],
    kind: ConfigurationKind,
    annotated: &Ident,
) -> Result<(Vec<Step>, HashSet<String>), Error> {
    let mut steps = Vec::new();
    let mut condition_names = HashSet::new();

    for (index, item_fn) in items
        .iter_mut()
        .filter_map(|item: &mut ImplItem| match item {
            ImplItem::Fn(item_fn) => Some(item_fn),
            _ => None,
        })
        .enumerate()
    {
        let provider_attr = ProviderAttr::remove_attributes(&mut item_fn.attrs)
            .map_err(|error| error.value)?
            .map(|attribute| attribute.value);
        let init_attr = InitAttr::remove_attributes(&mut item_fn.attrs)
            .map_err(|error| error.value)?
            .map(|attribute| attribute.value);
        let conditional_on_property =
            ConditionalOnPropertyAttr::remove_attributes(&mut item_fn.attrs)
                .map_err(|error| error.value)?
                .map(|attribute| attribute.value);

        let is_provider = provider_attr.is_some() || conditional_on_property.is_some();

        if is_provider && init_attr.is_some() {
            return Err(Error::new(
                item_fn.span(),
                "a method cannot declare both a provider and an initializer",
            ));
        }

        if is_provider {
            if matches!(item_fn.sig.output, ReturnType::Default) {
                return Err(Error::new(
                    item_fn.span(),
                    "a `#[provider]` method has to return the instance it registers",
                ));
            }

            let mut provider_attr = provider_attr;
            if let Some(attr) = provider_attr.as_mut() {
                collect_condition_names(&mut attr.conditional, annotated, &mut condition_names);
            }

            let autowired = read_autowired(&mut item_fn.sig)?;
            item_fn.attrs.push(parse_quote! { #[allow(dead_code)] });

            steps.push(expand_provider(
                index,
                item_fn,
                autowired,
                provider_attr,
                conditional_on_property,
            )?);

            continue;
        }

        if let Some(mut init_attr) = init_attr {
            if !kind.supports_init() {
                return Err(Error::new(
                    item_fn.span(),
                    format!(
                        "an `#[init]` method is only supported in a `#[configuration]`, \
                         not in an `#[{}]`",
                        kind.macro_name()
                    ),
                ));
            }

            if !matches!(item_fn.sig.output, ReturnType::Default) {
                return Err(Error::new(
                    item_fn.span(),
                    "an `#[init]` method does not return a value",
                ));
            }

            collect_condition_names(&mut init_attr.conditional, annotated, &mut condition_names);

            let autowired = read_autowired(&mut item_fn.sig)?;
            item_fn.attrs.push(parse_quote! { #[allow(dead_code)] });

            steps.push(expand_init(index, item_fn, autowired, init_attr)?);
        }
    }

    Ok((steps, condition_names))
}

/// Collects the simple names of the conditions of a step, which are the ones
/// the generated code can emit as a function of the configuration.
///
/// # Arguments
///
/// * `conditions` - The conditions to read, which are qualified in place.
/// * `annotated` - The type the configuration is generated for.
/// * `names` - The names that are collected.
fn collect_condition_names(
    conditions: &mut [ExprPath],
    annotated: &Ident,
    names: &mut HashSet<String>,
) {
    for condition in conditions {
        qualify_self(condition, annotated);

        let condition = condition.to_token_stream().to_string();
        if !condition.contains("::") {
            names.insert(condition);
        }
    }
}

/// Reads the `#[autowired]` attribute of every parameter of a method.
///
/// # Arguments
///
/// * `sig` - The signature of the method.
fn read_autowired(sig: &mut Signature) -> Result<Vec<Option<AutowiredAttr>>, Error> {
    let mut autowired = Vec::new();

    for input in sig.inputs.iter_mut() {
        if let FnArg::Typed(typed) = input {
            autowired.push(get_autowired_attr(typed)?);
        }
    }

    Ok(autowired)
}

/// Reads the `#[autowired]` attribute of a parameter of a method.
///
/// # Arguments
///
/// * `pat_type` - The parameter the attribute is read from.
fn get_autowired_attr(pat_type: &mut PatType) -> Result<Option<AutowiredAttr>, Error> {
    let Some(attribute) =
        AutowiredAttr::remove_attributes(&mut pat_type.attrs).map_err(|error| error.value)?
    else {
        return Ok(None);
    };

    Ok(attribute.value.into())
}

/// Returns the name a provider or a parameter is registered or resolved under
/// when it does not declare one.
///
/// The name is the one of the method or of the parameter in camel case, so that
/// `fn application_event_publisher()` provides `applicationEventPublisher` and
/// `security_properties: SecurityProperties` resolves `securityProperties`. A
/// leading underscore only marks a name that is not used, so it is not part of
/// the name.
///
/// # Arguments
///
/// * `name` - The name of the method or of the parameter.
fn default_singleton_name(name: &str) -> LitStr {
    let name = crate::util::name::field_name_to_singleton_name(name.trim_start_matches('_'));

    LitStr::new(&name, Span::call_site())
}

/// Expands a provider into the step that registers the instance it returns.
///
/// # Arguments
///
/// * `index` - The position of the method in the implementation block.
/// * `item_fn` - The method that declares the provider.
/// * `autowired_attrs` - The `#[autowired]` attributes of the parameters.
/// * `provider_attr` - The `#[provider]` attribute of the method.
/// * `conditional_on_property_attr` - The `#[conditional_on_property]` attribute
///   of the method.
fn expand_provider(
    index: usize,
    item_fn: &ImplItemFn,
    autowired_attrs: Vec<Option<AutowiredAttr>>,
    mut provider_attr: Option<ProviderAttr>,
    conditional_on_property_attr: Option<ConditionalOnPropertyAttr>,
) -> Result<Step, Error> {
    let ident = item_fn.sig.ident.clone();
    let is_async = if item_fn.sig.asyncness.is_some() {
        quote! { .await }
    } else {
        quote! {}
    };

    let order = provider_order(&provider_attr)?;

    let conditional = expand_conditional(
        provider_attr
            .as_ref()
            .map(|attr| attr.conditional.as_slice())
            .unwrap_or(&[]),
    );

    let conditional_on_property = conditional_on_property_attr.as_ref().map(|attr| {
        let name = &attr.name;
        let having_value = &attr.having_value;

        // The property is read from the environment of the application, which
        // is where the configuration file, the environment variables and the
        // command line arguments of the process meet, and its value is compared
        // as the text the environment resolved for it.
        quote! {
            let __property = match ctx
                .get_singleton_option_with_name::<
                    ::std::sync::Arc<
                        dyn ::next_web_core::env::ConfigurableEnvironment
                    >
                >(::next_web_context::APPLICATION_ENVIRONMENT_SINGLETON_NAME)
            {
                ::core::option::Option::Some(__environment) => {
                    use ::next_web_core::env::PropertyResolver as _;

                    __environment.get_property(#name)
                }
                ::core::option::Option::None => ::core::option::Option::None,
            };

            if __property.as_deref() != ::core::option::Option::Some(#having_value) {
                break 'gen;
            }
        }
    });

    let (ref_mut_cx_stmts, ref_cx_stmts, args) = expand_arguments(&item_fn.sig, autowired_attrs)?;

    let result_unwrap = provider_result_unwrap(&item_fn.sig);
    let instance = quote! {
        let instance = #ident( #(#args),* ) #is_async #result_unwrap;
    };

    let singleton_name = provider_attr
        .as_mut()
        .and_then(|attr| attr.name.take())
        .filter(|name| !name.value().is_empty())
        .unwrap_or_else(|| default_singleton_name(&item_fn.sig.ident.to_string()));
    let register_singleton = quote! { ctx.insert_singleton_with_name(instance, #singleton_name); };

    Ok(Step {
        index,
        order,
        body: quote! {
            'gen: {

                #item_fn

                #conditional_on_property
                #conditional

                #(#ref_mut_cx_stmts)*

                #(#ref_cx_stmts)*

                #instance
                #register_singleton
            }
        },
    })
}

/// Expands an initializer into the step that runs it.
///
/// # Arguments
///
/// * `index` - The position of the method in the implementation block.
/// * `item_fn` - The method that declares the initializer.
/// * `autowired_attrs` - The `#[autowired]` attributes of the parameters.
/// * `init_attr` - The `#[init]` attribute of the method.
fn expand_init(
    index: usize,
    item_fn: &ImplItemFn,
    autowired_attrs: Vec<Option<AutowiredAttr>>,
    init_attr: InitAttr,
) -> Result<Step, Error> {
    let ident = item_fn.sig.ident.clone();
    let is_async = if item_fn.sig.asyncness.is_some() {
        quote! { .await }
    } else {
        quote! {}
    };

    let order = match &init_attr.order {
        Some(order) => order.clone(),
        None => default_order()?,
    };

    let conditional = expand_conditional(&init_attr.conditional);

    let (ref_mut_cx_stmts, ref_cx_stmts, args) = expand_arguments(&item_fn.sig, autowired_attrs)?;

    Ok(Step {
        index,
        order,
        body: quote! {
            'gen: {

                #item_fn

                #conditional

                #(#ref_mut_cx_stmts)*

                #(#ref_cx_stmts)*

                #ident( #(#args),* ) #is_async;
            }
        },
    })
}

/// Expands the conditions of a step into the guard that skips it.
///
/// # Arguments
///
/// * `conditions` - The conditions of the step.
fn expand_conditional(conditions: &[ExprPath]) -> TokenStream2 {
    if conditions.is_empty() {
        return quote! {};
    }

    let conditions = conditions.iter();

    quote! {
        #[allow(unused_parens)]
        if !( #(#conditions(ctx))&&* ) {
            break 'gen;
        }
    }
}

/// Resolves the parameters of a step from the context.
///
/// Returns the statements that need the context mutably, the statements that
/// borrow the instances those statements stored, and the names the method is
/// called with.
///
/// # Arguments
///
/// * `sig` - The signature of the method.
/// * `autowired_attrs` - The `#[autowired]` attributes of the parameters.
fn expand_arguments(
    sig: &Signature,
    autowired_attrs: Vec<Option<AutowiredAttr>>,
) -> Result<(Vec<Stmt>, Vec<Stmt>, Vec<Ident>), Error> {
    let mut ref_mut_cx_stmts = Vec::new();
    let mut ref_cx_stmts = Vec::new();
    let mut args = Vec::new();

    // The references the context lends to the step, paired with their
    // mutability, which the rules of the borrow are checked against.
    let mut references: Vec<BorrowedReference> = Vec::new();

    for (index, (input, autowired_attr)) in sig
        .inputs
        .iter()
        .filter_map(|input| match input {
            FnArg::Typed(typed) => Some(typed),
            FnArg::Receiver(_) => None,
        })
        .zip(autowired_attrs)
        .enumerate()
    {
        let default_name = match input.pat.as_ref() {
            Pat::Ident(pat) => default_singleton_name(&pat.ident.to_string()),
            _ => LitStr::new("", Span::call_site()),
        };

        let (name, default) = match autowired_attr {
            Some(AutowiredAttr { name, default }) => (
                name.filter(|name| !name.value().is_empty())
                    .unwrap_or(default_name),
                default.unwrap_or_default(),
            ),
            None => (default_name, false),
        };

        let (ty, is_reference, is_mut) = match input.ty.as_ref() {
            Type::Reference(reference) => (
                reference.elem.as_ref(),
                true,
                reference.mutability.is_some(),
            ),
            ty => (ty, false, false),
        };

        if !is_reference {
            let arg = Ident::new(&format!("owned{index}"), Span::call_site());

            let resolve = if default {
                quote! {
                    let #arg = ctx
                        .resolve_option_with_name::<#ty>(#name)
                        .unwrap_or_default();
                }
            } else {
                quote! { let #arg = ctx.resolve_with_name::<#ty>(#name); }
            };

            push_resolve_stmts(
                ResolveOneValue::Owned {
                    resolve: parse2::<Stmt>(resolve)?,
                },
                &mut ref_mut_cx_stmts,
                &mut ref_cx_stmts,
            );

            args.push(arg);

            continue;
        }

        if default {
            return Err(Error::new(
                input.ty.span(),
                "a reference parameter borrows an instance of the context, so it cannot \
                 fall back to a default; make the parameter owned",
            ));
        }

        let arg = Ident::new(&format!("ref{index}"), Span::call_site());

        // The instance a reference parameter borrows has to exist before it can
        // be lent, so it is created first, when the context does not hold it
        // yet. A reference borrows an instance the context holds, and not a
        // value it resolved for the caller, so the step sees the instance the
        // other steps and the application see.
        let create_single = parse_quote! {
            ctx.try_just_create_singleton_with_name::<#ty>(#name);
        };

        let get_singleton = if is_mut {
            parse_quote! { let #arg = ctx.get_singleton_mut_with_name::<#ty>(#name); }
        } else {
            parse_quote! { let #arg = ctx.get_singleton_with_name::<#ty>(#name); }
        };

        references.push((input.ty.span(), is_mut));

        push_resolve_stmts(
            ResolveOneValue::Ref {
                create_single,
                get_singleton,
            },
            &mut ref_mut_cx_stmts,
            &mut ref_cx_stmts,
        );

        args.push(arg);
    }

    check_reference_borrows(&references)?;

    Ok((ref_mut_cx_stmts, ref_cx_stmts, args))
}

/// Collects the steps of a configuration into the code that runs them.
///
/// The steps are collected with the order they declare, sorted, and called in
/// that order. The order is an expression, so a step can be given the order of a
/// constant, and the steps of the same order keep the order they were declared
/// in.
///
/// # Arguments
///
/// * `steps` - The steps of the configuration.
fn gen_code(steps: Vec<Step>) -> TokenStream2 {
    let indices = steps.iter().map(|step| step.index).collect::<Vec<_>>();
    let pushed_indices = indices.iter();
    let matched_indices = indices.iter();
    let orders = steps.iter().map(|step| &step.order);
    let bodies = steps.iter().map(|step| &step.body);

    quote! {
        // The steps are collected with the order they declare, so that the
        // order of a step can be the value of a constant of the application,
        // and they run from the lowest order to the highest one.
        let mut __steps: ::std::vec::Vec<(i32, usize)> = ::std::vec::Vec::new();

        #(
            __steps.push((#orders, #pushed_indices));
        )*

        __steps.sort_by_key(|(order, _)| *order);

        for (_, __step) in __steps {
            match __step {
                #(
                    #matched_indices => {
                        #bodies
                    }
                )*
                _ => {}
            }
        }
    }
}

/// Returns the default order of a step, which is the default of
/// [`Ordered`](::next_web_core::Ordered).
fn default_order() -> Result<Expr, Error> {
    syn::parse_str(DEFAULT_ORDER).map_err(|_| {
        Error::new(
            Span::call_site(),
            "the default order of a configuration is a whole number",
        )
    })
}

/// Returns the order a provider runs in.
///
/// # Arguments
///
/// * `provider_attr` - The `#[provider]` attribute of the method.
fn provider_order(provider_attr: &Option<ProviderAttr>) -> Result<Expr, Error> {
    match provider_attr.as_ref().and_then(|attr| attr.order.as_ref()) {
        Some(order) => Ok(order.clone()),
        None => default_order(),
    }
}

/// Returns the names a `before` or an `after` attribute of a configuration
/// declares.
///
/// # Errors
///
/// Returns an error when a name is empty, because an empty name can never name a
/// configuration.
///
/// # Arguments
///
/// * `names` - The names the attribute declares.
/// * `attribute` - The attribute the names were declared in, which the error
///   names.
fn configuration_names(names: Vec<LitStr>, attribute: &str) -> Result<Vec<LitStr>, Error> {
    for name in &names {
        if name.value().trim().is_empty() {
            return Err(Error::new(
                name.span(),
                format!("a name of the `{attribute}` attribute must not be empty"),
            ));
        }
    }

    Ok(names)
}

/// Rewrites the `Self` of a condition into the annotated type, so that a
/// condition declared as `Self::enabled` is called on the type of the
/// configuration.
///
/// # Arguments
///
/// * `condition` - The condition to rewrite.
/// * `annotated` - The type the configuration is generated for.
fn qualify_self(condition: &mut ExprPath, annotated: &Ident) {
    if let Some(first) = condition.path.segments.first_mut() {
        if first.ident == "Self" {
            first.ident = Ident::new(&annotated.to_string(), first.span());
        }
    }
}

/// Checks that the annotated item can be a configuration class.
///
/// The block has to be an inherent implementation, because the configuration is
/// generated as a type of its own, and the methods of the block cannot take
/// `self`, because the framework calls them with the dependencies they declare
/// as their parameters.
///
/// # Errors
///
/// Returns an error when the block implements a trait, or when one of its
/// methods takes a `self` parameter.
///
/// # Arguments
///
/// * `item_impl` - The annotated implementation block.
/// * `kind` - The kind of configuration class the block declares.
fn safety_check(item_impl: &ItemImpl, kind: ConfigurationKind) -> Result<(), Error> {
    // 1. Check if the trait has been implemented
    if let Some(trait_name) = match item_impl.trait_.as_ref() {
        Some((_, path, _)) => path.segments.last().map(|s| s.ident.to_string()),
        None => None,
    } {
        return Err(Error::new(
            Span::call_site(),
            format!(
                "`{}` macros are not supported for trait '{}' implementation",
                kind.macro_name(),
                trait_name
            ),
        ));
    }

    // 2. Traverse all methods and check if there is a self parameter
    for item in item_impl.items.iter() {
        if let ImplItem::Fn(item_fn) = item {
            for fn_arg in item_fn.sig.inputs.iter() {
                if let FnArg::Receiver(receiver) = fn_arg {
                    return Err(Error::new(
                        receiver.span(),
                        "the method of a configuration cannot have a self parameter",
                    ));
                }
            }
        }
    }

    Ok(())
}

fn extract_result_ok_type(ty: &Type) -> Option<Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };

    let segment = type_path.path.segments.last()?;
    if segment.ident != "Result" {
        return None;
    }

    let syn::PathArguments::AngleBracketed(generic_args) = &segment.arguments else {
        return None;
    };

    generic_args.args.iter().find_map(|arg| match arg {
        syn::GenericArgument::Type(ty) => Some(ty.clone()),
        _ => None,
    })
}

fn provider_result_unwrap(sig: &Signature) -> TokenStream2 {
    match &sig.output {
        ReturnType::Type(_, ty) if extract_result_ok_type(ty.as_ref()).is_some() => quote! { ? },
        _ => quote! {},
    }
}

#[cfg(test)]
mod tests {
    use from_attr::FromAttr;
    use quote::ToTokens;
    use syn::parse_str;

    use super::{extract_result_ok_type, provider_result_unwrap};
    use crate::web::attrs::auto_configuration_attr::{AutoConfigurationAttr, ProviderAttr};

    #[test]
    fn the_attributes_of_a_provider_are_read() {
        let attributes = ProviderAttr::from_tokens(quote::quote! {
            name = "value", conditional = [Self::enabled], order = 12
        })
        .unwrap();

        assert_eq!(
            attributes.name.map(|name| name.value()),
            Some(String::from("value"))
        );
        assert_eq!(attributes.conditional.len(), 1);
        assert_eq!(
            attributes
                .order
                .map(|order| order.to_token_stream().to_string()),
            Some(String::from("12"))
        );
    }

    #[test]
    fn the_order_of_a_provider_is_an_expression() {
        let attributes = ProviderAttr::from_tokens(quote::quote! {
            order = i32::MIN
        })
        .unwrap();

        assert_eq!(
            attributes
                .order
                .map(|order| order.to_token_stream().to_string()),
            Some(String::from("i32 :: MIN"))
        );
    }

    #[test]
    fn the_attributes_of_a_configuration_are_read() {
        let attributes = AutoConfigurationAttr::from_tokens(quote::quote! {
            name = "testAutoConfiguration",
            order = 20,
            conditional = [Self::enabled, enabled_in_the_module]
        })
        .unwrap();

        assert_eq!(
            attributes.name.map(|name| name.value()),
            Some(String::from("testAutoConfiguration"))
        );
        assert_eq!(
            attributes
                .order
                .map(|order| order.to_token_stream().to_string()),
            Some(String::from("20"))
        );
        assert_eq!(attributes.conditional.len(), 2);
    }

    #[test]
    fn a_configuration_without_attributes_is_read() {
        let attributes = AutoConfigurationAttr::from_tokens(quote::quote! {}).unwrap();

        assert!(attributes.name.is_none());
        assert!(attributes.order.is_none());
        assert!(attributes.conditional.is_empty());
    }

    #[test]
    fn a_value_of_the_wrong_type_is_reported() {
        let attributes = ProviderAttr::from_tokens(quote::quote! {
            name = 12
        });

        assert!(attributes.is_err());
    }

    #[test]
    fn extract_result_ok_type_returns_inner_type() {
        let ty = parse_str::<syn::Type>("std::result::Result<String, BoxError>").unwrap();
        let ok_ty = extract_result_ok_type(&ty).unwrap();

        assert_eq!(ok_ty.to_token_stream().to_string(), "String");
    }

    #[test]
    fn provider_result_unwrap_adds_question_mark_for_result() {
        let sig =
            parse_str::<syn::Signature>("fn redis_template() -> Result<String, BoxError>").unwrap();

        assert_eq!(provider_result_unwrap(&sig).to_string(), "?");
    }

    #[test]
    fn provider_result_unwrap_ignores_plain_return_type() {
        let sig = parse_str::<syn::Signature>("fn redis_template() -> String").unwrap();

        assert!(provider_result_unwrap(&sig).is_empty());
    }
}
