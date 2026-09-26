use from_attr::FromAttr;
use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse_quote, punctuated::Punctuated, Expr, Fields, ItemStruct, Lit, LitStr, Meta, Token,
};

use crate::{util::logic::Logic, web::attrs::properties_attr::PropertiesAttr};

/// Returns whether the expression is the path of one of the functions that
/// bind the instance of the annotated struct.
///
/// # Arguments
///
/// * `expr` - The expression to test.
/// * `function` - The name of the function the expression has to name.
fn is_bind(expr: &Expr, function: &str) -> bool {
    expr.to_token_stream().to_string().replace(' ', "") == format!("Self::{function}")
}

/// Ensures that the `#[singleton]` (or `#[singleowner]`) attribute of the
/// annotated struct binds it to the `Properties` trait, so that the properties
/// of the struct can be resolved like the properties of any other singleton.
///
/// The attribute is left untouched when the struct is not declared as a
/// singleton, in which case the struct has to be registered by the application
/// itself. The name of the singleton is returned when the attribute declares
/// one.
///
/// # Arguments
///
/// * `attr` - The attribute to extend.
fn ensure_properties_bind(attr: &mut syn::Attribute) -> syn::Result<Option<String>> {
    if !attr.path().is_ident("singleton") && !attr.path().is_ident("singleowner") {
        return Ok(None);
    }

    let attr_path = attr.path().clone();
    let mut metas = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
    let mut singleton_name = None;
    let mut binds_index = None;

    for (index, meta) in metas.iter_mut().enumerate() {
        match meta {
            Meta::NameValue(name_value) if name_value.path.is_ident("name") => {
                if let Expr::Lit(expr_lit) = &name_value.value {
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        singleton_name = Some(lit_str.value());
                    }
                }
            }
            Meta::NameValue(name_value) if name_value.path.is_ident("binds") => {
                if !matches!(name_value.value, Expr::Array(_)) {
                    return Err(syn::Error::new_spanned(
                        &name_value.value,
                        "The `binds` attribute must be an array expression",
                    ));
                }

                binds_index = Some(index);
            }
            _ => {}
        }
    }

    let index = match binds_index {
        Some(index) => index,
        None => {
            metas.push(parse_quote!(binds = []));

            metas.len() - 1
        }
    };

    let Meta::NameValue(binds) = &mut metas[index] else {
        unreachable!("the meta of the binds is a name value");
    };
    let Expr::Array(array) = &mut binds.value else {
        unreachable!("the binds of a struct are an array");
    };

    for function in ["into_properties"] {
        if array.elems.iter().any(|elem| is_bind(elem, function)) {
            continue;
        }

        let bind: Expr = match function {
            "into_properties" => parse_quote!(Self::into_properties),
            _ => parse_quote!(Self::into_configuration_properties),
        };

        array.elems.push(bind);
    }

    *attr = parse_quote!(#[#attr_path(#metas)]);

    Ok(singleton_name)
}

/// Turns the `#[key = "..."]` attribute of a field into the `#[serde(rename =
/// "...")]` attribute serde reads, and removes the other `#[key]` attributes.
///
/// The attribute is how a field is bound from a property whose name is not the
/// name of the field. Serde is the deserializer of the properties, so the name
/// is passed on to it.
///
/// # Arguments
///
/// * `item_struct` - The struct the fields of which are renamed.
fn apply_key_renames(item_struct: &mut ItemStruct) -> syn::Result<()> {
    let Fields::Named(fields) = &mut item_struct.fields else {
        return Err(syn::Error::new(
            item_struct.ident.span(),
            "Only named fields are supported",
        ));
    };

    for field in &mut fields.named {
        let key = field
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("key"))
            .map(|attr| {
                let name_value = attr.meta.require_name_value()?;

                match &name_value.value {
                    Expr::Lit(lit) => match &lit.lit {
                        Lit::Str(key) => Ok(key.clone()),
                        lit => Err(syn::Error::new_spanned(
                            lit,
                            "The key of a field is a string",
                        )),
                    },
                    value => Err(syn::Error::new_spanned(
                        value,
                        "The key of a field is a string",
                    )),
                }
            })
            .transpose()?;

        field.attrs.retain(|attr| !attr.path().is_ident("key"));

        if let Some(key) = key {
            field.attrs.push(parse_quote!(#[serde(rename = #key)]));
        }
    }

    Ok(())
}

pub fn impl_macro_configuration_properties(
    attr: TokenStream,
    mut item_struct: ItemStruct,
) -> TokenStream {
    let expanded = Logic::generate(|| {
        let attr = PropertiesAttr::from_tokens(attr.to_owned().into())?;

        let required_derives = vec!["Clone", "Deserialize"];
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

        if !matches!(item_struct.fields, Fields::Named(_)) {
            return Err(syn::Error::new(
                item_struct.ident.span(),
                "Only named fields are supported",
            ));
        }

        let prefix = attr.prefix;
        let dynamic = attr.dynamic;
        let struct_ident = item_struct.ident.clone();
        let dynamic_struct_ident = format_ident!("_Dynamic{}", struct_ident);

        // The name of a field is the name of the property it is bound from, so
        // a name the attribute macro is asked to use is passed on to serde,
        // which is the deserializer of the properties.
        apply_key_renames(&mut item_struct)?;

        let mut singleton_name = None;

        if dynamic {
            let Fields::Named(fields) = &mut item_struct.fields else {
                unreachable!("the fields of the struct are named");
            };

            if fields
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

            // The dynamic properties are bound by the implementation below,
            // which cannot be expressed as a serde attribute.
            let dynamic_field: syn::Field =
                parse_quote!(#[serde(skip)] _dynamic: Option<#dynamic_struct_ident>);
            fields.named.push(dynamic_field);
        }

        // Ensure the singleton and singleowner attributes contain the binds
        // that make the properties of the struct discoverable.
        for attr in &mut item_struct.attrs {
            if let Some(name) = ensure_properties_bind(attr)? {
                singleton_name = Some(name);
            }
        }

        // Fall back to the default singleton name when no explicit name is
        // present.
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

        // The dynamic properties are the properties below the prefix that are
        // grouped by a key, which the binder reads without the fields of the
        // struct being bound a second time.
        let dynamic_binding = if dynamic {
            quote! {
                Ok(instance.with_dynamic(binder.bind_dynamic::<Self>()))
            }
        } else {
            quote! { Ok(instance) }
        };

        let dynamic_properties_fn = if dynamic {
            quote! {
                /// Returns the properties below the prefix of this type, grouped
                /// by the keys that hold them.
                pub fn dynamic_properties(&self) -> Option<&::std::collections::HashMap<String, Self>> {
                    self._dynamic.as_ref().map(|var| &var.0)
                }

                /// Returns this instance with the given dynamic properties.
                fn with_dynamic(
                    mut self,
                    dynamic: Option<::std::collections::HashMap<String, Self>>,
                ) -> Self {
                    self._dynamic = dynamic.map(#dynamic_struct_ident);
                    self
                }
            }
        } else {
            quote! {}
        };

        let expanded = quote! {
            #item_struct

            #dynamic_support


            impl ::next_web_core::context::properties::ConfigurationProperties for #struct_ident {
                fn name(&self) -> &'static str {
                    #singleton_name
                }

                fn prefix(&self) -> &'static str {
                    #prefix
                }

                fn bind(
                    environment: &dyn ::next_web_core::env::ConfigurableEnvironment,
                ) -> ::std::result::Result<Self, ::next_web_core::env::BindError> {
                    let binder = ::next_web_core::env::Binder::new(environment, #prefix);
                    let instance = binder.bind::<Self>()?;

                    #dynamic_binding
                }

                fn register(
                    &self,
                    ctx: &mut dyn ::next_web_core::ApplicationContext,
                    environment: &dyn ::next_web_core::env::ConfigurableEnvironment,
                ) -> ::std::result::Result<(), ::std::boxed::Box<dyn ::std::error::Error>> {
                    use ::next_web_context::ApplicationContextExt as _;

                    let instance = <Self as ::next_web_core::context::properties::ConfigurationProperties>::bind(environment)?;
                    ctx.insert_singleton_with_name(instance, #singleton_name);

                    Ok(())
                }
            }


            impl #struct_ident {
                #dynamic_properties_fn
            }

            // The properties of the struct are discovered by the framework, so
            // that the application does not have to list them itself. The
            // provider hands out an empty instance, which binds the properties
            // of the environment when they are registered.
            ::next_web_core::register_provider!(
                <::next_web_context::Provider<
                    ::std::boxed::Box<dyn ::next_web_core::context::properties::ConfigurationProperties>
                > as ::core::convert::From<_>>::from(
                    ::next_web_context::singleton(|_: &mut dyn ::next_web_core::ApplicationContext| {
                        ::std::boxed::Box::new(<#struct_ident as ::core::default::Default>::default())
                            as ::std::boxed::Box<dyn ::next_web_core::context::properties::ConfigurationProperties>
                    })
                    .name(#singleton_name)
                )
            );
        };

        // println!("expanded: {}", expanded.to_string());

        Ok(expanded.into())
    });

    expanded
}
