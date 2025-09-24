extern crate proc_macro;

use crate::common::retry::impl_macro_retry;
use crate::data::builder::impl_macro_builder;
use crate::data::field_name::impl_macro_field_name;
use crate::data::get_set::impl_macro_get_set;
use data::desensitized::impl_macro_desensitized;
use proc_macro::TokenStream;
use syn::parse_macro_input;
use syn::DeriveInput;
use syn::ItemFn;

mod web;
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

#[doc = ""]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn Retryable(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemFn);
    impl_macro_retry(attr, item)
}

// web

#[doc = ""]
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn RequestMapping(args: TokenStream, input: TokenStream) -> TokenStream {
    crate::web::routing::with_method(None, args, input)
}

macro_rules! method_macro {
    ($method:ident, $variant:ident) => {
        
        #[doc = ""]
        #[proc_macro_attribute]
        #[allow(non_snake_case)]
        pub fn $method(args: TokenStream, input: TokenStream) -> TokenStream {
            crate::web::routing::with_method(Some(crate::web::routing::Method::$variant), args, input)
        }
    };
}

method_macro!(GetMapping,       Get);
method_macro!(PostMapping,      Post);
method_macro!(PutMapping,       Put);
method_macro!(DeleteMapping,    Delete);
method_macro!(PatchMapping,     Patch);
method_macro!(AnyMapping,       Any);


#[doc = ""]
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Scheduled(args: TokenStream, input: TokenStream) -> TokenStream {
    crate::web::scheduled::impl_macro_scheduled(args, input)
}