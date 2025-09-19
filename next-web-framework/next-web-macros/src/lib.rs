extern crate proc_macro;

use crate::common::retry::impl_macro_retry;
use crate::data::builder::impl_macro_builder;
use crate::data::field_name::impl_macro_field_name;
use crate::data::get_set::impl_macro_get_set;
use crate::database::db_mapper::impl_macro_db_mapper;
use crate::singleton::find::impl_macro_find_singleton;
use data::desensitized::impl_macro_desensitized;
use proc_macro::TokenStream;
use syn::parse_macro_input;
use syn::DeriveInput;
use syn::ItemFn;

mod attrs;
mod common;
mod data;
mod database;
mod singleton;
mod util;

#[proc_macro_derive(GetSet, attributes(get_set))]
pub fn get_set(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    impl_macro_get_set(&input)
}

#[proc_macro_derive(FieldName)]
pub fn field_name(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    impl_macro_field_name(&input)
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    impl_macro_builder(&input)
}

#[proc_macro_derive(Desensitized, attributes(de))]
pub fn desensitized(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    impl_macro_desensitized(&input)
}

// database
#[proc_macro_derive(DbMapper, attributes(d))]
pub fn all_constructor(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let stream = impl_macro_db_mapper(&input);
    stream.into()
}


#[doc = ""]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn Retryable(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemFn);
    impl_macro_retry(attr, item)
}

#[doc = ""]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn Find(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemFn);
    impl_macro_find_singleton(attr, item)
}