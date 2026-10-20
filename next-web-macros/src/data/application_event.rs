//! Implementation of the `#[derive(ApplicationEvent)]` derive macro.
//!
//! An event of an application is a value that the event system clones, publishes
//! and hands to the listeners that support it. The trait it implements asks for
//! four methods, and three of them are the same for every event: the type of the
//! event, the source of it, and the type of the source. Writing them by hand is
//! boilerplate that hides the one thing that matters — the data of the event —
//! and it is where a mistake is made silently: an event that reports a type that
//! is not its own passes the filter of a listener and is then ignored by it.
//!
//! ```rust,ignore
//! #[derive(Clone, ApplicationEvent)]
//! struct OrderPlaced {
//!     id: String,
//! }
//! ```
//!
//! The macro generates:
//!
//! - `event_type` and `source_type`, both the type of the annotated value;
//! - `source`, the value itself, so a listener that reads the source downcasts the
//!   event.
//!
//! It does not generate `timestamp`. The trait asks for a method and not for a
//! field, so a generated method would report the time it is called and not the time
//! the event happened; the default of the trait (no timestamp) is used instead, and
//! an event that carries the time it happened keeps it in a field:
//!
//! ```rust,ignore
//! #[derive(Clone, ApplicationEvent)]
//! struct OrderPlaced {
//!     id: String,
//!     occurred_at: SystemTime,
//! }
//! ```
//!
//! # What the annotated value has to be
//!
//! - `Clone`, because every event of the system is cloneable;
//! - `'static`, because the type of an event is a [`TypeId`].
//!
//! # The path of the trait
//!
//! The trait is written as `::next_web_context::ApplicationEvent`, which is the
//! crate that defines it, so the crate that annotates a value depends on
//! `next-web-context` — which the framework re-exports as `next_web::context`.
//! A crate that depends on the framework only writes the path of the trait with a
//! `use`, and the derive names the crate that defines it, so both work.
//!
//! [`TypeId`]: std::any::TypeId

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, Error, spanned::Spanned};

/// Returns the code of the `ApplicationEvent` implementation of the annotated
/// value.
///
/// # Arguments
///
/// * `input` - The value the derive was written on.
pub fn impl_macro_application_event(input: &DeriveInput) -> Result<TokenStream2, Error> {
    let name = &input.ident;

    if !input.generics.params.is_empty() {
        return Err(Error::new(
            input.generics.span(),
            "`ApplicationEvent` cannot be derived for a value with type parameters or a \
             lifetime: the type of an event is a `TypeId`, and a `TypeId` is only available \
             for a `'static` type",
        ));
    }

    // The two paths the generated code needs, so that a crate that depends on the
    // framework and a crate that depends on the context alone both compile.
    Ok(quote! {
        impl ::next_web_context::ApplicationEvent for #name {
            fn source(&self) -> &dyn ::std::any::Any {
                self
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::impl_macro_application_event;

    #[test]
    fn the_implementation_names_the_source_as_the_value_itself() {
        let input: syn::DeriveInput = parse_quote! {
            struct OrderPlaced {
                id: String,
            }
        };

        let generated = impl_macro_application_event(&input)
            .expect("the derive of a value without parameters is generated")
            .to_string();

        assert!(generated.contains("impl :: next_web_context :: ApplicationEvent for OrderPlaced"));
        assert!(generated.contains("fn source"));
        assert!(
            !generated.contains("fn event_type"),
            "the type of the event is the default of the trait, so the generated code does not \
             report a type of its own: {generated}"
        );
    }

    #[test]
    fn a_value_with_type_parameters_is_refused() {
        let input: syn::DeriveInput = parse_quote! {
            struct OrderPlaced<T> {
                id: T,
            }
        };

        assert!(impl_macro_application_event(&input).is_err());
    }
}
