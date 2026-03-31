use from_attr::FromAttr;
use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse_quote, punctuated::Punctuated, Expr, Field, Fields, ItemStruct, LitStr, Meta, Token,
};

use crate::{
    util::{extract_type::extract_option_inner_type, field_type::FieldType, logic::Logic},
    web::attrs::properties_attr::PropertiesAttr,
};

fn is_into_properties_bind(expr: &Expr) -> bool {
    expr.to_token_stream().to_string().replace(' ', "") == "Self::into_properties"
}

fn ensure_properties_bind(attr: &mut syn::Attribute) -> syn::Result<Option<String>> {
    if !attr.path().is_ident("singleton") && !attr.path().is_ident("singleowner") {
        return Ok(None);
    }

    let attr_path = attr.path().clone();
    let mut metas = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
    let mut singleton_name = None;
    let mut has_binds = false;

    for meta in &mut metas {
        match meta {
            Meta::NameValue(name_value) if name_value.path.is_ident("name") => {
                if let Expr::Lit(expr_lit) = &name_value.value {
                    if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                        singleton_name = Some(lit_str.value());
                    }
                }
            }
            Meta::NameValue(name_value) if name_value.path.is_ident("binds") => {
                has_binds = true;

                let Expr::Array(array) = &mut name_value.value else {
                    return Err(syn::Error::new_spanned(
                        &name_value.value,
                        "The `binds` attribute must be an array expression",
                    ));
                };

                if !array.elems.iter().any(is_into_properties_bind) {
                    array.elems.push(parse_quote!(Self::into_properties));
                }
            }
            _ => {}
        }
    }

    if !has_binds {
        metas.push(parse_quote!(binds = [Self::into_properties]));
    }

    *attr = parse_quote!(#[#attr_path(#metas)]);

    Ok(singleton_name)
}

pub fn impl_macro_properties(attr: TokenStream, mut item_struct: ItemStruct) -> TokenStream {
    let expanded = Logic::generate(|| {
        let attr = PropertiesAttr::from_tokens(attr.to_owned().into())?;

        let required_derives = vec!["Debug", "Clone", "Deserialize"];
        let existing_derives = &item_struct
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("derive"))
            .filter_map(|attr| {
                let meta = attr.meta.require_list().ok()?;
                Some(meta.tokens.to_string())
            })
            .collect::<Vec<_>>();
        for required in required_derives {
            if !existing_derives.iter().any(|d| d.contains(required)) {
                return Err(syn::Error::new(
                    item_struct.ident.span(),
                    format!("Missing required derive: {}", required),
                ));
            }
        }

        let prefix_expr = attr.prefix;

        let prefix = prefix_expr.to_token_stream().to_string().replace("\"", "");
        let dynamic = attr.dynamic;
        let struct_ident = item_struct.ident.clone();
        let dynamic_struct_ident = format_ident!("_Dynamic{}", struct_ident);

        let fields = match &item_struct.fields {
            Fields::Named(fields_named) => &fields_named.named,
            _ => {
                return Err(syn::Error::new(
                    item_struct.ident.span(),
                    "Only named fields are supported",
                ))
            }
        };

        // Generate field access and assignment code.
        let common_fields = fields
            .iter()
            .filter(|field| field.ident.is_some())
            .filter_map(|field| {
                let field_name = field.ident.as_ref()?;
                let field_type = &field.ty;

                // Check whether the field type is Option<T>.
                let (is_option, inner_type) = match extract_option_inner_type(field_type) {
                    Some(ty) => (true, ty),
                    None => (false, field_type.clone()),
                };

                // Read the key attribute or fall back to the field name.
                let key_name = field
                    .attrs
                    .iter()
                    .find(|attr| attr.path().is_ident("key"))
                    .and_then(|attr| {
                        attr.meta.require_name_value().ok().and_then(|meta| {
                            if let syn::Expr::Lit(expr_lit) = &meta.value {
                                if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                                    return Some(lit_str.value());
                                }
                            }
                            None
                        })
                    })
                    .unwrap_or_else(|| field_name.to_string());

                // Build the final property key.
                let key = if prefix.is_empty() {
                    key_name
                } else {
                    format!("{}.{}", prefix, key_name)
                };
                let key_str = LitStr::new(&key, field_name.span());

                // Detect String-like fields, including Option<String>.
                let is_string_type = FieldType::is_string(&inner_type);

                // Generate the expression that reads the value from properties.
                let extract_value_expr = if is_string_type {
                    // String fields also accept numeric property values.
                    quote! {
                        || -> Option<String> {
                            // Try the string value first.
                            if let Some(s) = properties.get_value::<String>(#key_str) {
                                return Some(s);
                            }

                            match properties.get_value::<i64>(#key_str) {
                                Some(s) => Some(s.to_string()),
                                None => match properties.get_value::<f64>(#key_str) {
                                    Some(s) => Some(s.to_string()),
                                    None => None,
                                }
                            }
                        }()
                    }
                } else {
                    // Non-string fields read directly via get_value.
                    quote! {
                        properties.get_value::<#inner_type>(#key_str)
                    }
                };

                // Generate the final field initialization for Option<T> or T.
                let field_init = if is_option {
                    quote! { #field_name: #extract_value_expr, }
                } else {
                    quote! {
                        #field_name: #extract_value_expr.unwrap_or_else(|| {
                            noting = true;
                            Default::default()
                        }),
                    }
                };

                Some(field_init)
            })
            .collect::<Vec<_>>();

        let dynamic_field = if dynamic {
            quote! {
                _dynamic: properties
                    .get_dynamic_value::<#struct_ident>(#prefix_expr)
                    .map(#dynamic_struct_ident),
            }
        } else {
            quote! {}
        };

        // Remove field-level key attributes from the generated struct.
        if let Fields::Named(fields_named) = &mut item_struct.fields {
            for field in &mut fields_named.named {
                field.attrs.retain(|attr| !attr.path().is_ident("key"));
            }

            if dynamic {
                if fields_named
                    .named
                    .iter()
                    .filter_map(|field| field.ident.as_ref())
                    .any(|ident| ident == "_dynamic")
                {
                    return Err(syn::Error::new(
                        struct_ident.span(),
                        "The `_dynamic` field is generated automatically for dynamic properties",
                    ));
                }

                let dynamic_field: Field =
                    parse_quote!(#[serde(skip)] _dynamic: Option<#dynamic_struct_ident>);
                fields_named.named.push(dynamic_field);
            }
        }
        let struct_ident = &item_struct.ident;

        // Ensure singleton and singleowner attributes contain the required binds.
        let mut singleton_name = None;
        for attr in &mut item_struct.attrs {
            if let Some(name) = ensure_properties_bind(attr)? {
                singleton_name = Some(name);
            }
        }

        // Fall back to the default singleton name when no explicit name is present.
        let singleton_name = if let Some(name) = singleton_name {
            name
        } else {
            crate::util::name::singleton_name(&struct_ident.to_string())
        };

        let singleton_name = LitStr::new(&singleton_name, struct_ident.span());

        let dynamic_support = if dynamic {
            quote! {
                #[derive(Debug, Clone)]
                struct #dynamic_struct_ident(pub ::std::collections::HashMap<String, #struct_ident>);
            }
        } else {
            quote! {}
        };

        let dynamic_properties_fn = if dynamic {
            quote! {
                pub fn dynamic_properties(&self) -> Option<&::std::collections::HashMap<String, Self>> {
                    self._dynamic.as_ref().map(|var| &var.0)
                }
            }
        } else {
            quote! {}
        };

        let expanded = quote! {
            #item_struct

            #dynamic_support

            #[next_web_core::async_trait]
            impl ::next_web_core::AutoRegister for #struct_ident {
                async fn register(
                    &self,
                    ctx: &mut ::next_web_core::context::application_context::ApplicationContext,
                    properties: & ::next_web_core::context::properties::ApplicationProperties,
                ) -> ::std::result::Result<(), ::std::boxed::Box<dyn ::std::error::Error + Send + Sync>> {
                    let mut noting = false;

                    let instance = Self {
                        #dynamic_field

                        #(#common_fields)*
                    };

                    if noting {
                        panic!("\nIncorrect assembly of properties! Struct: {} \n", stringify!(#struct_ident));
                    }

                    ctx.insert_singleton_with_name(instance, #singleton_name);
                    Ok(())
                }

                fn name(&self) -> &'static str {
                    #singleton_name
                }
            }

            impl ::next_web_core::context::properties::Properties for #struct_ident {}

            impl #struct_ident {
                #dynamic_properties_fn

                fn into_properties(self) -> ::std::boxed::Box<dyn ::next_web_core::context::properties::Properties> {
                    ::std::boxed::Box::new(self)
                }
            }
        };

        Ok(expanded.into())
    });

    expanded
}
