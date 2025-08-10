use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{Data, Ident, Meta};

use crate::utils::type_info::TypeInfo;

pub(crate) fn impl_macro_get_set(input: &syn::DeriveInput) -> proc_macro::TokenStream {
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data_struct) => &data_struct.fields,
        _ => panic!("error: #[derive(GetSet)] is only supported for structs"),
    };

    let get_methods = generate_get_methods(fields);
    let set_methods = generate_set_methods(fields);

    let token_stream = quote! {
        impl #name {
            #( #get_methods )*
            #( #set_methods )*
        }
    };

    token_stream.into()
}

/// 生成getter方法
fn generate_get_methods(fields: &syn::Fields) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .filter(|field| {
            // 过滤条件：有标识符、不以_开头、没有skip属性或skip不包含GET
            field.ident.is_some()
                && !field.ident.as_ref().unwrap().to_string().starts_with('_')
                && !should_skip(field, "GET")
        })
        .map(|field| {
            let value = field.ident.as_ref().unwrap();
            let func_name = Ident::new(
                &format!("get_{}", value),
                Span::call_site(),
            );
            
            match &field.ty {
                syn::Type::Path(type_path) => {
                    if let Some(first_segment) = type_path.path.segments.first() {
                        match first_segment.ident.to_string().as_str() {
                            "Option" => generate_option_getter(&field.ty, &func_name, value),
                            numeric if TypeInfo::is_number(numeric) => {
                                quote! { pub fn #func_name(&self) -> #numeric { self.#value } }
                            }
                            "String" => {
                                quote! { pub fn #func_name(&self) -> &str { &self.#value } }
                            }
                            _ => {
                                quote! { pub fn #func_name(&self) -> &#field.ty { &self.#value } }
                            }
                        }
                    } else {
                        quote! { pub fn #func_name(&self) -> &#field.ty { &self.#value } }
                    }
                }
                _ => quote! { pub fn #func_name(&self) -> &#field.ty { &self.#value } },
            }
        })
        .collect()
}

/// 生成setter方法
fn generate_set_methods(fields: &syn::Fields) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .filter(|field| {
            // 过滤条件：有标识符、不以_开头、没有skip属性或skip不包含SET
            field.ident.is_some()
                && !field.ident.as_ref().unwrap().to_string().starts_with('_')
                && !should_skip(field, "SET")
        })
        .map(|field| {
            let value = field.ident.as_ref().unwrap();
            let func_name = Ident::new(
                &format!("set_{}", value),
                Span::call_site(),
            );
            quote! {
                pub fn #func_name(&mut self, value: #field.ty) {
                    self.#value = value;
                }
            }
        })
        .collect()
}

/// 检查字段是否应该跳过生成方法
fn should_skip(field: &syn::Field, method_type: &str) -> bool {
    field.attrs.iter().any(|attr| {
        if attr.path().is_ident("skip") {
            match &attr.meta {
                Meta::Path(_) => true, // #[skip] 跳过所有方法
                Meta::NameValue(meta) => {
                    if let syn::Expr::Lit(expr_lit) = &meta.value {
                        let value = expr_lit.lit.to_token_stream().to_string();
                        value.to_uppercase() == format!("\"{}\"", method_type)
                    } else {
                        false
                    }
                }
                _ => false,
            }
        } else {
            false
        }
    })
}

/// 生成Option类型的getter方法
fn generate_option_getter(
    field_type: &syn::Type,
    func_name: &Ident,
    value: &Ident,
) -> proc_macro2::TokenStream {
    if let syn::Type::Path(type_path) = field_type {
        if let Some(first_segment) = type_path.path.segments.first() {
            if let syn::PathArguments::AngleBracketed(args) = &first_segment.arguments {
                if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                    match inner_type {
                        syn::Type::Path(inner_path) => {
                            if let Some(inner_segment) = inner_path.path.segments.first() {
                                let inner_type_name = inner_segment.ident.to_string();
                                if TypeInfo::is_number(&inner_type_name) {
                                    return quote! {
                                        pub fn #func_name(&self) -> Option<#inner_type> {
                                            self.#value.clone()
                                        }
                                    };
                                } else if TypeInfo::is_string(&inner_type_name) {
                                    return quote! {
                                        pub fn #func_name(&self) -> Option<&str> {
                                            self.#value.as_deref()
                                        }
                                    };
                                } else {
                                    return quote! {
                                        pub fn #func_name(&self) -> Option<&#inner_type> {
                                            self.#value.as_ref()
                                        }
                                    };
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    
    // 默认实现
    quote! {
        pub fn #func_name(&self) -> #field_type {
            self.#value.clone()
        }
    }
}