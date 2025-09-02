use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, Fields, Ident};

use crate::util::field_type::FieldType;

pub(crate) fn impl_macro_builder(input: &syn::DeriveInput) -> TokenStream {
    let struct_name = &input.ident;

    match &input.data {
        Data::Struct(_) => (),
        _ => panic!("error: #[derive(Builder)] is only supported for structs"),
    };

    let builder_name = Ident::new(&format!("{}Builder", struct_name), struct_name.span());
    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(named_fields) => &named_fields.named,
            _ => panic!("Builder only supports structs with named fields"),
        },
        _ => panic!("Builder only supports structs"),
    };

    // Generate fields or corresponding Option<T> fields
    let builder_fields = fields
        .iter()
        .filter(|val| val.ident.is_some())
        .filter(|val| {
            !val.ident
                .as_ref()
                .map(|s| s.to_string().starts_with("_"))
                .unwrap_or_default()
        })
        .map(|field| {
            let field_name = &field.ident;
            let field_type = &field.ty;

            if FieldType::is_option(field_type) {
                quote! {
                    #field_name: #field_type
                }
            } else {
                quote! {
                    #field_name: ::std::option::Option<#field_type>
                }
            }
        });

    // Setter method for generating fields
    let setter_methods = fields
        .iter()
        .filter(|val| val.ident.is_some())
        .map(|field| {
            let field_name = &field.ident.as_ref().unwrap();
            let field_type =
                FieldType::extract_option_inner_type(&field.ty).unwrap_or(field.ty.clone());

            let is_into = field
                .attrs
                .iter()
                .filter(|val| val.path().is_ident("builder"))
                .any(|va1| {
                    let mut flag = false;
                    va1.parse_nested_meta(|meta| {
                        if meta.path.is_ident("into") {
                            flag = true;
                        }
                        Ok(())
                    })
                    .unwrap_or_default();

                    flag
                });

            match is_into {
                true => {
                    quote! {
                        pub fn #field_name(mut self, #field_name: impl Into<#field_type>) -> Self {
                            self.#field_name = Some(#field_name.into());
                            self
                        }
                    }
                }
                false => {
                    quote! {
                        pub fn #field_name(mut self, #field_name: #field_type) -> Self {
                            self.#field_name = Some(#field_name);
                            self
                        }
                    }
                }
            }
        })
        .collect::<Vec<_>>();

    // Generate build method
let build_method = {
    let field_initializers = fields.iter().map(|field| {
        let field_name = &field.ident;
        let is_option = FieldType::is_option(&field.ty);
        
        let setter = match FieldConfig::from_field(field) {
            FieldConfig::WithDefaultFunction(function_ident) => {
                if is_option {
                    quote! { self.#field_name.map(Some).unwrap_or_else(|| #function_ident()) }
                } else {
                    quote! { self.#field_name.unwrap_or_else(|| #function_ident()) }
                }
            }
            FieldConfig::WithDefaultTrait => {
                if is_option {
                    quote! { self.#field_name.map(Some).unwrap_or_else(|| Some(Default::default())) }
                } else {
                    quote! { self.#field_name.unwrap_or_else(|| Default::default()) }
                }
            }
            FieldConfig::Required => {
                if is_option {
                    quote! { self.#field_name.map(Some).take().ok_or(concat!(stringify!(#field_name), " is not set"))? }
                } else {
                    quote! { self.#field_name.take().ok_or(concat!(stringify!(#field_name), " is not set"))? }
                }
            }
        };

        quote! { #field_name: #setter }
    });

    quote! {
        pub fn build(mut self) -> Result<#struct_name, &'static str> {
            Ok(#struct_name {
                #( #field_initializers ),*
            })
        }
    }
};

    // In the builder function, all fields default to None
    let builer_fields = fields
        .iter()
        .filter(|val| val.ident.is_some())
        .map(|field| {
            let field_name = &field.ident;
            quote! { #field_name: None }
        });

    // Generate a complete Builder implementation
    let expanded = quote! {

        #[doc = concat!(
            "Builder for [`", stringify!(#struct_name), "`].\n\n",
            "This builder allows you to set fields incrementally and then build the final [`",
            stringify!(#struct_name), "`] instance using `.build()`.\n\n"
        )]
        pub struct #builder_name {
            #( #builder_fields ),*
        }

        impl #builder_name {

            #[doc = concat!("Create a new [`", stringify!(#builder_name), "`].")]
            pub fn builder() -> Self {
                Self {
                    #( #builer_fields ),*
                }
            }

            #( #setter_methods )*

            #build_method
        }

    };

    // println!("expanded: {}", expanded.to_string());

    expanded.into()
}

enum FieldConfig {
    WithDefaultFunction(syn::Ident),
    WithDefaultTrait,
    Required,
}

impl FieldConfig {
    fn from_field(field: &syn::Field) -> Self {
        let mut result = Self::Required;
        for attr in &field.attrs {
            if attr.path().is_ident("builder") {
                attr.parse_nested_meta(|meta| {
                        match meta.path.get_ident() {
                        Some(ident) => {
                            if ident == "default" {
                                 match meta.value() {
                                        Ok(lit) => {
                                            let function_str: syn::LitStr = lit.parse()?;
                                            let function_ident = syn::Ident::new(&function_str.value(), function_str.span());
                                            result =  Self::WithDefaultFunction(function_ident);
                                        },
                                        Err(_) => result = Self::WithDefaultTrait,
                                    }
                            }
                        },
                        None => {}
                    }
                    Ok(())
                  }).unwrap();
            }
        }
        
        result
    }
}
