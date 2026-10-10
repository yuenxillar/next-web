//! Implementation of the `#[order]` attribute macro.
//!
//! The macro implements `Ordered` for the annotated item with the order the
//! attribute is given, so a type declares its place without an implementation
//! block of its own:
//!
//! ```rust,ignore
//! #[order(12)]
//! struct App;
//!
//! #[order(-1)]
//! struct Early;
//! ```
//!
//! The macro is applied to a type definition (a struct, an enum, or a union) or
//! to an implementation block, and the order it is given is written into the
//! `order` method of the trait.
//!
//! The path of the trait is `::next_web_core::Ordered`, which is the crate that
//! defines it, so the crate that annotates a type depends on `next-web-core`.

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{spanned::Spanned, Error, Generics, Item};

/// Returns the code of the `Ordered` implementation of the annotated item, with
/// the item it was annotated on.
///
/// # Arguments
///
/// * `order` - The order the attribute was given.
/// * `item` - The item the attribute was annotated on.
pub fn generate_order(order: TokenStream2, item: Item) -> Result<TokenStream2, Error> {
    let order: syn::Expr = if order.is_empty() {
        return Err(Error::new(
            Span::call_site(),
            "#[order] requires the order, for example #[order(12)]",
        ));
    } else {
        syn::parse2(order)?
    };

    let (self_ty, generics) = ordered_type(&item)?;
    let (impl_generics, _, where_clause) = generics.split_for_impl();

    Ok(quote! {
        #item

        impl #impl_generics ::next_web_core::Ordered for #self_ty #where_clause {
            fn order(&self) -> i32 {
                #order
            }
        }
    })
}

/// Returns the type the implementation is written for, and the generics the
/// implementation is declared with.
///
/// # Arguments
///
/// * `item` - The item the attribute was annotated on.
fn ordered_type(item: &Item) -> Result<(TokenStream2, &Generics), Error> {
    match item {
        Item::Struct(item) => {
            let ident = &item.ident;
            let (_, ty_generics, _) = item.generics.split_for_impl();

            Ok((quote!(#ident #ty_generics), &item.generics))
        }
        Item::Enum(item) => {
            let ident = &item.ident;
            let (_, ty_generics, _) = item.generics.split_for_impl();

            Ok((quote!(#ident #ty_generics), &item.generics))
        }
        Item::Union(item) => {
            let ident = &item.ident;
            let (_, ty_generics, _) = item.generics.split_for_impl();

            Ok((quote!(#ident #ty_generics), &item.generics))
        }
        // An implementation block already names the type it is written for, so
        // the type of the block is the type the order is implemented for.
        Item::Impl(block) => {
            let self_ty = &block.self_ty;

            Ok((quote!(#self_ty), &block.generics))
        }
        other => Err(Error::new(
            other.span(),
            "#[order] is applied to a struct, an enum, a union, or an implementation block",
        )),
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::generate_order;

    /// Returns the code the macro generates for an item.
    ///
    /// # Arguments
    ///
    /// * `order` - The order the attribute is given.
    /// * `item` - The item the attribute is annotated on.
    fn code_of(
        order: proc_macro2::TokenStream,
        item: proc_macro2::TokenStream,
    ) -> proc_macro2::TokenStream {
        generate_order(order, syn::parse2(item).expect("the item is parsable"))
            .expect("the order is generated")
    }

    #[test]
    fn implements_the_order_of_a_struct() {
        let code = code_of(quote! { 12 }, quote! { struct App; });

        zyn::assert_tokens_contain!(code, r#"impl :: next_web_core :: Ordered for App"#);
        zyn::assert_tokens_contain!(code, r#"fn order (& self) -> i32 { 12 }"#);
    }

    #[test]
    fn keeps_the_attributes_of_the_item() {
        let code = code_of(quote! { 12 }, quote! { #[derive(Clone)] pub struct App; });

        zyn::assert_tokens_contain!(code, r#"# [derive (Clone)]"#);
        zyn::assert_tokens_contain!(code, r#"impl :: next_web_core :: Ordered for App"#);
    }

    #[test]
    fn implements_a_negative_order() {
        let code = code_of(quote! { -1 }, quote! { struct Early; });

        zyn::assert_tokens_contain!(code, r#"fn order (& self) -> i32 { - 1 }"#);
    }

    #[test]
    fn carries_the_generics_of_the_item() {
        let code = code_of(quote! { 5 }, quote! { struct Wrapper<T> { value: T } });

        zyn::assert_tokens_contain!(
            code,
            r#"impl < T > :: next_web_core :: Ordered for Wrapper < T >"#
        );
    }

    #[test]
    fn implements_the_order_of_an_enum() {
        let code = code_of(quote! { 1 }, quote! { enum Kind { First } });

        zyn::assert_tokens_contain!(code, r#"impl :: next_web_core :: Ordered for Kind"#);
    }

    #[test]
    fn implements_the_order_of_an_implementation_block() {
        let code = code_of(
            quote! { 7 },
            quote! {
                impl Service {
                    fn name(&self) -> &'static str {
                        "service"
                    }
                }
            },
        );

        zyn::assert_tokens_contain!(code, r#"impl :: next_web_core :: Ordered for Service"#);
        zyn::assert_tokens_contain!(code, r#"fn name (& self) -> & 'static str"#);
    }

    #[test]
    fn an_order_is_required() {
        let error = generate_order(quote! {}, syn::parse_quote! { struct App; })
            .expect_err("the order is required");

        assert!(error.to_string().contains("requires the order"), "{error}");
    }

    #[test]
    fn an_item_that_is_not_a_type_is_reported() {
        let error = generate_order(
            quote! { 1 },
            syn::parse_quote! {
                fn handler() {}
            },
        )
        .expect_err("a function cannot be ordered");

        assert!(
            error.to_string().contains("struct, an enum, a union"),
            "{error}"
        );
    }
}
