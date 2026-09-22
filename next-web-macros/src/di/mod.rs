//! Attribute macros that turn an item into a provider of the dependency
//! injection context.
//!
//! The macros are re-exported as `next_web::macros::bind::{singleton,
//! transient, singleowner}`. They all take the same arguments and only differ
//! in the [`Scope`] of the provider they create:
//!
//! - `singleton` creates a provider whose instance is constructed once and that
//!   can be resolved by reference or, when the type implements `Clone`, by
//!   value.
//! - `transient` creates a provider whose instance is constructed on every
//!   resolution.
//! - `singleowner` creates a provider whose instance is constructed once and
//!   that is only resolved by reference.
//!
//! The generated code refers to `::next_web_context` by default; the path can
//! be changed with `#[resource(path = ...)]`.

mod commons;
mod field_or_argument_attr;
mod impl_fn_or_enum_variant_attr;
mod item_enum_gen;
mod item_fn_gen;
mod item_impl_gen;
mod item_struct_gen;
mod resource_attr;
mod struct_or_function_attr;
mod value_attr;

use from_attr::FromAttr;
use next_web_context::Scope;
use proc_macro::TokenStream;
use syn::{parse_macro_input, spanned::Spanned, Item};

use crate::di::struct_or_function_attr::StructOrFunctionAttr;

/// Generates the provider of the annotated item for the given scope.
pub(crate) fn generate(attr: TokenStream, item: TokenStream, scope: Scope) -> TokenStream {
    let attr = match StructOrFunctionAttr::from_tokens(attr.into()) {
        Ok(attr) => attr,
        Err(err) => return err.to_compile_error().into(),
    };

    let item = parse_macro_input!(item as Item);

    let result = match item {
        Item::Struct(item_struct) => item_struct_gen::generate(attr, item_struct, scope),
        Item::Enum(item_enum) => item_enum_gen::generate(attr, item_enum, scope),
        Item::Fn(item_fn) => item_fn_gen::generate(attr, item_fn, scope),
        Item::Impl(item_impl) => item_impl_gen::generate(attr, item_impl, scope),
        _ => Err(syn::Error::new(
            item.span(),
            "expected `struct` or `enum` or `function` or `impl block`",
        )),
    };

    result.unwrap_or_else(|e| e.to_compile_error()).into()
}
