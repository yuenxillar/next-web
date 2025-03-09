use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{Data, Ident, Meta};

pub(crate) fn impl_macro_get_set(input: &syn::DeriveInput) -> proc_macro::TokenStream {
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data_struct) => &data_struct.fields,
        _ => panic!("error: #[derive(GetSet)] is only supported for structs"),
    };

    let get_methods = fields
        .iter()
        .filter(|item| item.ident.is_some())
        .filter(|item| !item.ident.clone().unwrap().to_string().starts_with("_"))
        .filter(|item: &&syn::Field| {
            item.attrs
                .iter()
                .filter(|item| item.path().is_ident("skip"))
                .collect::<Vec<_>>()
                .get(0)
                .map(|item| {
                    if let Meta::Path(_path) = &item.meta {
                        return false;
                    }
                    if let Meta::NameValue(meta) = &item.meta {
                        if let syn::Expr::Lit(expr_lit) = &meta.value {
                            let name = expr_lit.lit.to_token_stream().to_string().to_uppercase();
                            if name.eq("\"GET\"") {
                                return false;
                            }
                        }
                    }
                    true
                })
                .unwrap_or(true)
        })
        .map(|field| {
            let value = &field.ident;
            let field_type = &field.ty;

            if !value.is_none() {
                let func_name = Ident::new(
                    &format!(
                        "get_{}",
                        value.clone().map(|f| f.to_string()).unwrap_or_default()
                    ),
                    Span::call_site().into(),
                );
                match &field_type {
                    syn::Type::Path(type_path) => {
                        if let Some(first_segment) = type_path.path.segments.get(0) {
                            match first_segment.ident.to_string().as_str() {
                                "Option" => {
                                    if let syn::PathArguments::AngleBracketed(var) =
                                        &first_segment.arguments
                                    {
                                        if let Some(syn::GenericArgument::Type(syn::Type::Path(
                                            var,
                                        ))) = var.args.get(0)
                                        {
                                            if let Some(r_type) = var.path.segments.get(0) {
                                                let s = r_type.ident.to_string();
                                                let return_type =
                                                    get_return_type(&s, &func_name, &field.ident);
                                                return return_type;
                                            }
                                        }
                                    }
                                    return quote! {
                                        pub fn #func_name(&self) -> &#field_type {
                                            &self.#value
                                        }
                                    };
                                }
                                "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16"
                                | "i32" | "i64" | "i128" | "isize" => {
                                    return quote! {
                                        pub fn #func_name(&self) -> #field_type {
                                            self.#value
                                        }
                                    };
                                }
                                "String" => {
                                    return quote! {
                                        pub fn #func_name(&self) -> &str {
                                            &self.#value
                                        }
                                    };
                                }
                                _ => {
                                    return quote! {
                                        pub fn #func_name(&self) -> &#field_type {
                                            &self.#value
                                        }
                                    };
                                }
                            }
                        }
                        return quote! {
                            pub fn #func_name(&self) -> &self.#field_type {
                                self.#value.clone()
                            }
                        };
                    }
                    _ => {
                        return quote! {
                            pub fn #func_name(&self) -> &self.#field_type {
                                self.#value.clone()
                            }
                        }
                    }
                }
            }
            return quote! {};
        })
        .collect::<Vec<_>>();

    let set_methods = fields
        .iter()
        .filter(|item| item.ident.is_some())
        .filter(|item| !item.ident.clone().unwrap().to_string().starts_with("_"))
        .filter(|item: &&syn::Field| {
            item.attrs
                .iter()
                .filter(|item| item.path().is_ident("skip"))
                .collect::<Vec<_>>()
                .get(0)
                .map(|item| {
                    if let Meta::Path(_path) = &item.meta {
                        return false;
                    }
                    if let Meta::NameValue(meta) = &item.meta {
                        if let syn::Expr::Lit(expr_lit) = &meta.value {
                            let name = expr_lit.lit.to_token_stream().to_string();
                            if name.to_uppercase().as_str().eq("\"SET\"") {
                                return false;
                            }
                        }
                    }

                    true
                })
                .unwrap_or(true)
        })
        .map(|field| {
            let value = &field.ident;
            let field_type = &field.ty;

            if !value.is_none() {
                let func_name = Ident::new(
                    &format!(
                        "set_{}",
                        value.clone().map(|f| f.to_string()).unwrap_or_default()
                    ),
                    Span::call_site().into(),
                );
                return quote! {
                    pub fn #func_name(&mut self, value: #field_type) {
                        self.#value = value;
                    }
                };
            }
            quote! {}
        })
        .collect::<Vec<_>>();

    let token_stream = quote! {
        impl #name {
            #( #get_methods )*

            #( #set_methods )*
        }
    };

    return token_stream.into();
}

use crate::utils::type_info::TypeInfo;
fn get_return_type(s: &str, func_name: &Ident, value: &Option<Ident>) -> proc_macro2::TokenStream {
    let info = Ident::new(s, Span::call_site());
    let option: syn::Path = syn::parse_quote! { Option };
    let mut generic_args: syn::AngleBracketedGenericArguments = syn::parse_quote! { <> };
    let type_ref: syn::TypeReference;

    if TypeInfo::is_number(s) {
        let type_t = syn::parse_quote! { #info };
        generic_args.args.push(syn::GenericArgument::Type(type_t));

        let return_type: syn::Type = syn::parse_quote! { #option #generic_args };
        return quote! {
            pub fn #func_name(&self) -> #return_type {
                self.#value.clone()
            }
        };
    } else if TypeInfo::is_string(s) {
        let type_t: syn::Type = syn::parse_quote! { str };
        type_ref = syn::parse_quote! { & #type_t };
    } else {
        let type_t: syn::Type = syn::parse_quote! { #info };
        let type_ref: syn::Type = syn::parse_quote! { & #type_t };
        generic_args
            .args
            .push(syn::GenericArgument::Type(type_ref.into()));
        let return_type: syn::Type = syn::parse_quote! { #option #generic_args};
        return quote! {
            pub fn #func_name(&self) -> #return_type {
                self.#value.as_ref()
            }
        };
    }

    generic_args
        .args
        .push(syn::GenericArgument::Type(type_ref.into()));
    let return_type: syn::Type = syn::parse_quote! { #option #generic_args };
    return quote! {
        pub fn #func_name(&self) -> #return_type {
            self.#value.as_deref()
        }
    };
}
