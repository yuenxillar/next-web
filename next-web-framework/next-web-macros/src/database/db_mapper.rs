use proc_macro::TokenStream;
use syn::Data;

pub(crate) fn impl_macro_db_mapper(input: &syn::DeriveInput) -> TokenStream {
    let struct_name = &input.ident;

    match &input.data {
        Data::Struct(_) => (),
        _ => panic!("error: #[derive(Builder)] is only supported for structs"),
    };

    TokenStream::new()
}
