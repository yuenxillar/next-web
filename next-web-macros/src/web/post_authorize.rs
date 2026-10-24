//! Implementation of the `#[post_authorize]` attribute macro.
//!
//! The macro turns the rule of a handler into Rust code that is evaluated after
//! the body of the handler produced its result, so a handler answers only a
//! request whose authentication satisfies a rule about the value the handler
//! computed.
//!
//! ```rust,ignore
//! #[post_authorize("result == authentication.name")]
//! #[get_mapping(path = "/me")]
//! async fn me() -> String { /* ... */ }
//!
//! #[post_authorize("result.owner == authentication.name and result.public")]
//! #[get_mapping(path = "/accounts/{id}")]
//! async fn account(Path(id): Path<String>) -> Json<Account> { /* ... */ }
//! ```
//!
//! # The rule is Rust
//!
//! The rule is not interpreted while the request is handled: the macro compiles
//! it into a Rust expression over two bindings, `result` (the value the handler
//! returned) and `authentication` (the authentication of the request), and the
//! compiler type-checks it together with the handler. A rule can therefore read a
//! field of the result, call a method of it, and compare it with anything Rust
//! compares with.
//!
//! The Spring Security spelling of a rule is understood as well, so
//! `has_role('ADMIN')`, `has_any_role('ADMIN', 'AUDITOR')`,
//! `has_authority('order:read')`, `and`, `or` and `not` mean what they mean
//! there: `has_role('ADMIN')` is compiled into `authentication.has_role("ADMIN")`,
//! `and` into `&&`, `or` into `||` and `not` into `!`.
//!
//! # Placement
//!
//! `#[post_authorize]` must be the outermost handler attribute, above the
//! `#[get_mapping]`/`#[post_mapping]`/... that registers the handler, exactly like
//! `#[pre_authorize]`. The routing macros rebuild the function and only re-emit
//! the attributes they receive, so an inner `#[post_authorize]` would be dropped.
//!
//! # Behaviour
//!
//! The annotation rewrites the handler so that it returns `impl IntoResponse` and
//! takes the guard of the request as its first argument, which is extracted before
//! the handler runs. The body of the handler runs as it is, and the rule is
//! evaluated against the value it returned: a request the rule refuses is answered
//! with the refusal of the application (and with `403 Forbidden` when the
//! application configured none), and a request it allows is answered with the
//! value.
//!
//! The body already ran when the rule is evaluated, so a handler that writes must
//! be guarded with `#[pre_authorize]` instead: what a refused request did is not
//! undone.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{ItemFn, LitStr, spanned::Spanned};

use crate::util::{field_type::FieldType, logic::Logic};

/// Expands the `#[post_authorize]` attribute of a handler.
///
/// # Arguments
///
/// * `macro_attrs` - The arguments of the attribute.
/// * `item_fn` - The handler the attribute is declared on.
pub fn impl_macro_post_authorize(macro_attrs: TokenStream, item_fn: ItemFn) -> TokenStream {
    Logic::generate(|| {
        let vis = &item_fn.vis;
        let attrs = &item_fn.attrs;
        let block = &item_fn.block;

        if item_fn.sig.asyncness.is_none() {
            return Err(syn::Error::new(
                item_fn.sig.span(),
                "Function must be declared as async",
            ));
        }

        if matches!(item_fn.sig.output, syn::ReturnType::Default) {
            return Err(syn::Error::new(
                item_fn.sig.output.span(),
                "Function has no return type. Cannot be used as handler",
            ));
        }

        let Some(rule) = parse_rule(macro_attrs)? else {
            // `#[post_authorize(ignore = true)]`: the handler is emitted
            // untouched, which is what lets an application turn the rule of a
            // handler off for a deployment without editing its body.
            let sig = &item_fn.sig;

            return Ok(quote! { #(#attrs)* #vis #sig #block });
        };

        // Rewriting the return type to `impl IntoResponse` lets the guard return
        // the refusal of the application regardless of the concrete type the
        // handler produces. The original `Result<_, _>` return type is re-applied
        // to the body, so that its error type is still inferable.
        let result_type = match &item_fn.sig.output {
            syn::ReturnType::Type(_, typ) if FieldType::is_result(typ) => quote!(: #typ),
            _ => TokenStream2::new(),
        };

        // A handler that answers with Json<T> computes a T, so the rule of
        // the handler is written against the value it serializes and not against
        // the wrapper that writes it. Any other return type is the value itself.
        let subject = subject_binding(&item_fn.sig.output);

        let mut sig = item_fn.sig.clone();
        sig.output = syn::parse_quote!(-> impl ::next_web::response::IntoResponse);
        sig.inputs.insert(
            0,
            syn::parse_quote!(
                __next_web_post_authorize: ::next_web::security::access::prepost::post_authorize::PostAuthorizeContext
            ),
        );

        Ok(quote! {
            #(#attrs)*
            #vis #sig {
                let __next_web_post_authorize_result #result_type = (async move #block).await;

                {
                    // The rule reads the value the handler produced and the
                    // authentication of the request. The bindings are the two
                    // names a rule is written against, and they are dropped
                    // before the response is built.
                    #[allow(unused_variables)]
                    let result = #subject;
                    #[allow(unused_variables)]
                    let authentication = __next_web_post_authorize.authentication();

                    if !(#rule) {
                        return ::next_web::response::IntoResponse::into_response(
                            __next_web_post_authorize.denied(),
                        );
                    }
                }

                ::next_web::response::IntoResponse::into_response(
                    __next_web_post_authorize_result,
                )
            }
        })
    })
}

/// Returns the expression the rule of the handler reads the result with.
///
/// A handler that answers with `Json<T>` (or the `ValidatedJson<T>` of the
/// framework) computes a `T`, so the rule of the handler is written against the
/// value it serializes: `result.owner` reads the field of the value, and not the
/// wrapper that writes it. Any other return type is the value itself.
///
/// # Arguments
///
/// * `output` - The return type of the handler.
fn subject_binding(output: &syn::ReturnType) -> TokenStream2 {
    let syn::ReturnType::Type(_, typ) = output else {
        return quote!(&__next_web_post_authorize_result);
    };
    let syn::Type::Path(type_path) = typ.as_ref() else {
        return quote!(&__next_web_post_authorize_result);
    };
    let Some(segment) = type_path.path.segments.last() else {
        return quote!(&__next_web_post_authorize_result);
    };

    match segment.ident.to_string().as_str() {
        "Json" | "ValidatedJson" => quote!(&__next_web_post_authorize_result.0),
        _ => quote!(&__next_web_post_authorize_result),
    }
}

/// Parses the arguments of the attribute into the rule of the handler.
///
/// Returns `Ok(None)` for `ignore = true`, which leaves the handler untouched.
///
/// # Arguments
///
/// * `macro_attrs` - The arguments of the attribute.
fn parse_rule(macro_attrs: TokenStream) -> Result<Option<TokenStream2>, syn::Error> {
    let tokens: TokenStream2 = macro_attrs.into();
    let source = tokens.to_string();

    if source.trim().is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "`#[post_authorize]` needs a rule, for example \
             `#[post_authorize(\"result.owner == authentication.name\")]`",
        ));
    }

    // `ignore = true`, which is the escape hatch the `#[pre_authorize]` attribute
    // has as well.
    if source.replace(' ', "").starts_with("ignore=") {
        let literal: LitStr = syn::parse_str(
            source
                .split_once('=')
                .map(|(_, value)| value.trim())
                .unwrap_or_default(),
        )?;
        if literal.value() != "true" {
            return Err(syn::Error::new(
                literal.span(),
                "`ignore` of `#[post_authorize]` is either `true` or absent",
            ));
        }

        return Ok(None);
    }

    let literal: LitStr = syn::parse_str(&source).map_err(|_| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "`#[post_authorize]` takes its rule as a string, for example \
             `#[post_authorize(\"result.owner == authentication.name\")]`",
        )
    })?;

    let rust = to_rust(&literal.value()).map_err(|message| {
        syn::Error::new(
            literal.span(),
            format!("the rule of `#[post_authorize]` {message}"),
        )
    })?;

    let expression: syn::Expr = syn::parse_str(&rust).map_err(|error| {
        syn::Error::new(
            literal.span(),
            format!(
                "the rule of `#[post_authorize]` is not a Rust expression: {error}\n\
                 the rule `{}` was read as `{rust}`",
                literal.value()
            ),
        )
    })?;

    Ok(Some(quote!(#expression)))
}

/// Translates a rule written in the Spring Security spelling into Rust.
///
/// The translation is what makes the rules of the two frameworks look alike:
/// `and` becomes `&&`, `or` becomes `||`, `not` becomes `!`, a single quoted
/// string becomes a Rust string, and the authority checks become method calls on
/// the authentication the rule reads. Everything the translation does not touch
/// is left for the Rust compiler.
///
/// # Arguments
///
/// * `rule` - The rule of the attribute.
fn to_rust(rule: &str) -> Result<String, String> {
    let characters = rule.chars().collect::<Vec<_>>();
    let mut rust = String::with_capacity(rule.len() + 16);
    let mut index = 0;

    while index < characters.len() {
        let character = characters[index];

        // A string of the rule, which is single quoted the way the rules of the
        // reference implementation are.
        if character == '\'' || character == '"' {
            let quote = character;
            index += 1;
            let mut value = String::new();
            loop {
                let Some(next) = characters.get(index).copied() else {
                    return Err(String::from("starts a string it does not end"));
                };
                index += 1;
                if next == quote {
                    break;
                }
                if next == '\\' {
                    let Some(escaped) = characters.get(index).copied() else {
                        return Err(String::from("ends with an escape"));
                    };
                    index += 1;
                    value.push(escaped);
                    continue;
                }
                value.push(next);
            }

            rust.push('"');
            for escaped in value.chars() {
                match escaped {
                    '"' => rust.push_str("\\\""),
                    '\\' => rust.push_str("\\\\"),
                    other => rust.push(other),
                }
            }
            rust.push('"');

            continue;
        }

        // An identifier of the rule.
        if character.is_alphabetic() || character == '_' {
            let start = index;
            while index < characters.len()
                && (characters[index].is_alphanumeric() || characters[index] == '_')
            {
                index += 1;
            }
            let identifier = characters[start..index].iter().collect::<String>();

            match identifier.as_str() {
                "and" => rust.push_str("&&"),
                "or" => rust.push_str("||"),
                "not" => rust.push('!'),
                "has_role" | "has_any_role" | "has_authority" | "has_any_authority" => {
                    // A rule that already calls the method of the authentication
                    // keeps the call it wrote, so `authentication.has_role(...)`
                    // and `has_role(...)` mean the same thing.
                    if !rust.trim_end().ends_with('.') {
                        rust.push_str("authentication.");
                    }
                    rust.push_str(&identifier);
                }
                "permit_all" | "deny_all" => {
                    // The call of the rule is replaced by the value it means, so
                    // the parentheses that follow it are skipped as well.
                    rust.push_str(if identifier == "permit_all" {
                        "true"
                    } else {
                        "false"
                    });
                    while characters.get(index).is_some_and(|next| next.is_whitespace()) {
                        index += 1;
                    }
                    if characters.get(index) == Some(&'(') {
                        index += 1;
                        while characters.get(index).is_some_and(|next| next.is_whitespace()) {
                            index += 1;
                        }
                        if characters.get(index) != Some(&')') {
                            return Err(String::from(
                                "calls permit_all or deny_all with an argument",
                            ));
                        }
                        index += 1;
                    }
                }
                _ => rust.push_str(&identifier),
            }

            continue;
        }

        rust.push(character);
        index += 1;
    }

    Ok(rust)
}

#[cfg(test)]
mod tests {
    use super::to_rust;

    #[test]
    fn a_rule_of_the_reference_implementation_is_translated() {
        assert_eq!(
            to_rust("has_role('ADMIN') and has_authority('order:read')")
                .expect("the rule is translated"),
            "authentication.has_role(\"ADMIN\") && authentication.has_authority(\"order:read\")"
        );
        assert_eq!(
            to_rust("not has_role('BANNED')").expect("the rule is translated"),
            "! authentication.has_role(\"BANNED\")"
        );
        assert_eq!(
            to_rust("permit_all()").expect("the rule is translated"),
            "true"
        );
        assert_eq!(
            to_rust("deny_all()").expect("the rule is translated"),
            "false"
        );
    }

    #[test]
    fn a_rule_over_the_result_is_left_to_rust() {
        assert_eq!(
            to_rust("result.owner == authentication.name").expect("the rule is translated"),
            "result.owner == authentication.name"
        );
        assert_eq!(
            to_rust("result.items[0].owner == authentication.name")
                .expect("the rule is translated"),
            "result.items[0].owner == authentication.name"
        );
        assert_eq!(
            to_rust("result.owner() == authentication.name").expect("the rule is translated"),
            "result.owner() == authentication.name"
        );
    }

    #[test]
    fn a_string_of_the_rule_is_a_rust_string() {
        assert_eq!(
            to_rust("result.name == 'max'").expect("the rule is translated"),
            "result.name == \"max\""
        );
        assert_eq!(
            to_rust(r#"result.name == "max""#).expect("the rule is translated"),
            "result.name == \"max\""
        );
    }

    #[test]
    fn a_rule_that_starts_a_string_it_does_not_end_is_refused() {
        assert!(to_rust("result.name == 'max").is_err());
    }
}
