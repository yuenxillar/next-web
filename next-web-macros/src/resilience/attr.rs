//! Parsing of the attribute arguments the five macros accept.
//!
//! All five share one grammar, so a reader who has learned one has learned them
//! all:
//!
//! ```text
//! #[pattern(SOURCE)]                          // a field, a static, any expression
//! #[pattern(name = "resource")]               // looked up in the process registry
//! #[pattern(SOURCE, fallback = EXPRESSION)]   // what to return when rejected
//! ```

use std::fmt;

use quote::quote;
use syn::{
    Expr, Ident, LitStr, Token,
    parse::{Parse, ParseStream},
};

/// Where the macro takes the pattern instance from.
pub(crate) enum Source {
    /// An expression evaluating to the instance, such as a field of `self`.
    Instance(Expr),
    /// The name the instance is registered under in the process registry.
    Name(LitStr),
}

/// The parsed attribute arguments.
pub(crate) struct ResilienceAttr {
    /// Where the instance comes from.
    pub(crate) source: Source,
    /// Expression to return when the pattern rejects the call.
    ///
    /// Without one, a rejection is converted into the guarded function's own
    /// error with `Into::into`, which requires a `From` implementation for the
    /// rejection type. With one, that conversion is never needed.
    pub(crate) fallback: Option<Expr>,
}

// Written by hand rather than derived: `syn` only implements `Debug` for its own
// syntax tree behind the `extra-traits` feature, which would slow every build of
// this crate down for a diagnostic that only matters to whoever is debugging a
// macro expansion.
impl fmt::Debug for ResilienceAttr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let source = match &self.source {
            Source::Instance(expression) => quote!(#expression).to_string(),
            Source::Name(name) => quote!(name = #name).to_string(),
        };

        f.debug_struct("ResilienceAttr")
            .field("source", &source)
            .field(
                "fallback",
                &self.fallback.as_ref().map(|expr| quote!(#expr).to_string()),
            )
            .finish()
    }
}

impl Parse for ResilienceAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.is_empty() {
            return Err(input.error(
                "a resilience attribute needs to say where the pattern instance comes from: \
                 pass an expression such as `self.limiters.login`, or `name = \"login\"` to \
                 look the instance up in the process registry",
            ));
        }

        let source = if opens_with_name(input) {
            // `name` is only the keyword argument on the far side of `=`; an
            // expression that merely begins with an identifier called `name` — a
            // field access, say — is left to the expression parser below.
            input.parse::<Ident>()?;
            input.parse::<Token![=]>()?;
            Source::Name(input.parse()?)
        } else {
            Source::Instance(input.parse()?)
        };

        let mut fallback = None;
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }

            let key = input.parse::<Ident>().map_err(|error| {
                syn::Error::new(error.span(), "expected `fallback = <expression>`")
            })?;
            if key != "fallback" {
                return Err(syn::Error::new(
                    key.span(),
                    format!(
                        "unknown argument `{key}`; the only accepted argument is \
                         `fallback = <expression>`"
                    ),
                ));
            }

            input.parse::<Token![=]>()?;
            if fallback.is_some() {
                return Err(syn::Error::new(
                    key.span(),
                    "`fallback` was given more than once",
                ));
            }
            fallback = Some(input.parse()?);
        }

        Ok(Self { source, fallback })
    }
}

/// Returns whether the attribute opens with `name = ...`.
fn opens_with_name(input: ParseStream<'_>) -> bool {
    let fork = input.fork();
    match fork.parse::<Ident>() {
        Ok(ident) => ident == "name" && fork.peek(Token![=]),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::{ResilienceAttr, Source};

    fn parse(tokens: proc_macro2::TokenStream) -> ResilienceAttr {
        syn::parse2(tokens).expect("the attribute should parse")
    }

    #[test]
    fn an_expression_names_the_instance() {
        let attr = parse(quote!(self.limiters.login));

        assert!(matches!(attr.source, Source::Instance(_)));
        assert!(attr.fallback.is_none());
    }

    #[test]
    fn a_name_selects_the_registry_form() {
        let attr = parse(quote!(name = "login"));

        match attr.source {
            Source::Name(name) => assert_eq!(name.value(), "login"),
            Source::Instance(_) => panic!("expected the registry form"),
        }
    }

    #[test]
    fn an_expression_may_begin_with_an_identifier_called_name() {
        // `name` here is a field, not the keyword argument.
        let attr = parse(quote!(name.limiter));

        assert!(matches!(attr.source, Source::Instance(_)));
    }

    #[test]
    fn a_fallback_follows_the_source() {
        let attr = parse(quote!(self.limiter, fallback = Ok(0)));

        assert!(attr.fallback.is_some());
    }

    #[test]
    fn a_trailing_comma_is_allowed() {
        let attr = parse(quote!(self.limiter, fallback = Ok(0),));

        assert!(attr.fallback.is_some());
    }

    #[test]
    fn a_missing_source_is_reported() {
        let error = syn::parse2::<ResilienceAttr>(quote!()).expect_err("expected an error");

        assert!(
            error
                .to_string()
                .contains("where the pattern instance comes from")
        );
    }

    #[test]
    fn an_unknown_argument_is_reported() {
        let error = syn::parse2::<ResilienceAttr>(quote!(self.limiter, limit = 5))
            .expect_err("expected an error");

        assert!(error.to_string().contains("unknown argument"));
    }

    #[test]
    fn a_duplicate_fallback_is_reported() {
        let error =
            syn::parse2::<ResilienceAttr>(quote!(self.limiter, fallback = Ok(1), fallback = Ok(2)))
                .expect_err("expected an error");

        assert!(error.to_string().contains("more than once"));
    }
}
