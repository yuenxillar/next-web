use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Data, Ident};

pub(crate) fn impl_macro_field_name(input: &syn::DeriveInput) -> TokenStream {
    let name = &input.ident;
    let fields = match &input.data {
        Data::Struct(data_struct) => &data_struct.fields,
        _ => panic!("error: #[derive(FieldName)] is only supported for structs"),
    };

    let fileds_names = fields
        .iter()
        .filter(|item| item.ident.is_some())
        .map(|item| item.ident.clone().unwrap())
        .map(|item| item.to_string())
        .filter(|item| !item.starts_with("_"))
        .collect::<Vec<String>>();

    let get_methods = fileds_names.iter().map(|item| {
        let method_name = Ident::new(&format!("field_{}", item), Span::call_site().into());
        return quote! {
            pub fn #method_name() -> &'static str {
                stringify!(#item)
            }
        };
    }).collect::<Vec<proc_macro2::TokenStream>>();

    let token_stream = quote! {
        impl #name {
            #( #get_methods )*
        }
    };
    return token_stream.into();
}
