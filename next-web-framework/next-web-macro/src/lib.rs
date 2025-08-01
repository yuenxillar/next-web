
extern crate proc_macro;

use crate::common::retry::impl_macro_retry;
use crate::data::builder::impl_macro_builder;
use crate::data::field_name::impl_macro_field_name;
use crate::data::get_set::impl_macro_get_set;
use crate::database::db_mapper::impl_macro_db_mapper;
use proc_macro::TokenStream;
use syn::parse_macro_input;
use syn::DeriveInput;

mod common;
mod data;
mod database;
mod utils;

#[proc_macro_derive(GetSet, attributes(skip))]
pub fn get_set(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let stream = impl_macro_get_set(&input);
    stream.into()
}


#[proc_macro_derive(FieldName)]
pub fn field_name(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let stream = impl_macro_field_name(&input);
    stream.into()
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let stream = impl_macro_builder(&input);
    stream.into()
}


// database
#[proc_macro_derive(DbMapper, attributes(d))]
pub fn all_constructor(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let stream = impl_macro_db_mapper(&input);
    stream.into()
}


#[proc_macro_attribute]
pub fn retry(attr: TokenStream, item: TokenStream) -> TokenStream {
    let stream = impl_macro_retry(attr, item);
    stream.into()
}