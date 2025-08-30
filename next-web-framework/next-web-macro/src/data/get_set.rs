use proc_macro2::Span;
use quote::quote;
use syn::{punctuated::Punctuated, Data, Ident, Meta, Token};

use crate::util::field_type::FieldType;

pub(crate) fn impl_macro_get_set(input: &syn::DeriveInput) -> proc_macro::TokenStream {
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data_struct) => &data_struct.fields,
        _ => panic!("error: #[derive(GetSet)] is only supported for structs"),
    };

    let get_methods = generate_get_methods(fields);
    let set_methods = generate_set_methods(fields);

    let expanded = quote! {

        impl #name {
            #( #get_methods )*
            #( #set_methods )*
        }
    };

    expanded.into()
}

// Generate Getter Methods
fn generate_get_methods(fields: &syn::Fields) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .filter(|field| check(field, SkipType::Get))
        .filter(|field| field.ident.is_some())
        .map(|field| {
            let value = field.ident.as_ref().unwrap();
            let fn_name = Ident::new(&format!("get_{}", value), Span::call_site());
            let field_type = &field.ty;

            match FieldType::from_type(field_type) {
                FieldType::String => quote! {
                    pub fn #fn_name(&self) -> &str {
                        self.#value.as_ref()
                    }
                },
                FieldType::Number | FieldType::Boolean => quote! {
                    pub fn #fn_name(&self) -> #field_type {
                        self.#value
                    }
                },
                FieldType::Option => generate_option_getter(field_type, &fn_name, value),
                _ => quote! {
                    pub fn #fn_name(&self) -> & #field_type {
                        & self.#value
                    }
                },
            }
        })
        .collect()
}

/// Generate setter methods
fn generate_set_methods(fields: &syn::Fields) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .filter(|field| check(field, SkipType::Set))
        .filter(|field| field.ident.is_some())
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let fn_name = Ident::new(&format!("set_{}", field_name), Span::call_site());
            let field_type = &field.ty;
            quote! {
                pub fn #fn_name(&mut self, #field_name: #field_type) {
                    self.#field_name = #field_name;
                }
            }
        })
        .collect()
}

#[derive(PartialEq, Eq)]
enum SkipType {
    Set,
    Get,
    All,
}

impl SkipType {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "skip_set" => SkipType::Set,
            "skip_get" => SkipType::Get,
            "skip" => SkipType::All,
            _ => panic!("SkipType error: invalid skip type: {}", s),
        }
    }
}

/// Check if the field should skip the generation method
fn check(field: &syn::Field, skip_type: SkipType) -> bool {
    field.ident.is_some()
        && !field
            .ident
            .as_ref()
            .map(|val| val.to_string().starts_with('_'))
            .unwrap_or(false)
        && (field.attrs.len() == 0
            || field.attrs.iter().all(|attr| {
                if attr.path().is_ident("get_set") {
                    return match &attr.meta {
                        Meta::List(list) => {
                            match list
                                .parse_args_with(Punctuated::<Ident, Token![,]>::parse_terminated)
                            {
                                Ok(args) => args.iter().all(|val| {
                                    let skip = SkipType::from_str(val.to_string().as_str());
                                    skip != skip_type && skip != SkipType::All
                                }),
                                _ => true,
                            }
                        }
                        _ => true,
                    };
                }
                true
            }))
}

/// Generate getter methods of Option type
fn generate_option_getter(
    field_type: &syn::Type,
    fn_name: &Ident,
    field_name: &Ident,
) -> proc_macro2::TokenStream {
    if let syn::Type::Path(type_path) = field_type {
        if let Some(first_segment) = type_path.path.segments.first() {
            if let syn::PathArguments::AngleBracketed(args) = &first_segment.arguments {
                if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                    return match FieldType::from_type(inner_type) {
                        FieldType::String => quote! {
                            pub fn #fn_name(&self) -> Option<&str> {
                                self.#field_name.as_deref()
                            }
                        },
                        FieldType::Boolean | FieldType::Number => quote! {
                            pub fn #fn_name(&self) -> Option<#inner_type> {
                                self.#field_name.clone()
                            }
                        },
                        FieldType::Option => panic!("error: Option<Option<T>> is not supported"),
                        _ => quote! {
                            pub fn #fn_name(&self) -> Option<& #inner_type> {
                                self.#field_name.as_ref()
                            }
                        },
                    };
                }
            }
        }
    }

    quote! {
        pub fn #fn_name(&self) -> #field_type {
            self.#field_name.clone()
        }
    }
}
