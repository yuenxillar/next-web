use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{punctuated::Punctuated, spanned::Spanned, Error, Meta};

use crate::util::{field_type::FieldType, logic::Logic};

pub fn impl_macro_required_args_constructor(input: &syn::DeriveInput) -> TokenStream {
    let expanded = Logic::generate(|| {
        let name = &input.ident;

        let data_struct = match &input.data {
            syn::Data::Struct(data_struct) => data_struct,
            _ => {
                return Err(Error::new(
                    input.span(),
                    "Only structs can be used with the `required_args_constructor` macro.",
                ))
            }
        };

        let (args, fields): (Vec<TokenStream2>, Vec<TokenStream2>) = data_struct
            .fields
            .iter()
            .filter(|field| {
                field
                    .ident
                    .as_ref()
                    .map(|s| !s.to_string().starts_with("_"))
                    .unwrap_or(false)
            })
            .map(|field| {
                let is_option = FieldType::is_option(&field.ty);
                let field_name = field.ident.as_ref();
                let field_type = &field.ty;

                let  (mut required, mut default, mut skip) = (false, false, false); 
                field.attrs.iter().for_each(|attr| {
                    if attr.path().is_ident("constructor") {
                        if let Meta::List(meta_list) = &attr.meta {
                          
                            let meta: Punctuated<Meta, syn::Token![,]> = match meta_list
                                .parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated)
                            {
                                Ok(nested_meta) => nested_meta,
                                Err(_) => return,
                            };

                            required = meta.iter().any(|meta| meta.path().is_ident("required"));
                            default = meta.iter().any(|meta| meta.path().is_ident("default"));
                            skip = meta.iter().any(|meta| meta.path().is_ident("skip"));
                        }
                    }
                });
                if skip {
                    return (quote! {}, quote! {#field_name: Default::default()});
                }

                if is_option {
                    if required {
                        (quote! {#field_name: #field_type}, quote! {#field_name})
                    }else {
                        (quote! {}, quote! {#field_name: None})
                    }
                } else {
                    (quote! {#field_name: #field_type}, quote! {#field_name})
                }
            })
            .collect::<Vec<_>>()
            .into_iter()
            .unzip();
        let stream = quote! {

            impl #name {

                pub fn from_args(
                    #(#args),*
                ) -> Self {
                    Self {
                        #(#fields),*
                    }
                }
            }
        };

        Ok(stream)
    });

    println!("expanded: {}", expanded.to_string());
    TokenStream::from(expanded)
}
