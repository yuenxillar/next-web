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
        .map(|item| item.ident.as_ref().map(|s| s.to_string()))
        .map(|item| item.unwrap_or_default())
        .filter(|item| !item.starts_with("_"))
        .collect::<Vec<_>>();

    let methods = fileds_names
        .iter()
        .map(|item| {
            let method_name = Ident::new(&format!("field_{}", item), Span::call_site().into());
            return quote! {
                pub fn #method_name() -> &'static str {
                    stringify!(#item)
                }
            };
        })
        .collect::<Vec<proc_macro2::TokenStream>>();

    let expanded = quote! {
        impl #name {
            #( #methods )*
        }
    };

    // println!("expanded: {}", expanded.to_string());

    expanded.into()
}
