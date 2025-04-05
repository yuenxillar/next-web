use crate::PropertiesAttr;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Fields, ItemStruct, LitStr};

pub fn generate(attr: PropertiesAttr, item_struct: ItemStruct) -> syn::Result<TokenStream> {
    println!("aaaaaaattr: {:?}", attr.prefix);
    println!("aaaaaaitem_struct: {:?}", item_struct.attrs);
    let required_derives = vec!["Debug", "Clone", "Deserialize", "Default"];

    let existing_derives = item_struct
        .clone()
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

    let prefix = attr.prefix.to_token_stream().to_string();
    let fields = match &item_struct.fields {
        Fields::Named(fields_named) => &fields_named.named,
        _ => {
            return Err(syn::Error::new(
                item_struct.ident.span(),
                "Only named fields are supported",
            ))
        }
    };

    let property_keys = fields.iter().filter_map(|field| {
        let field_name = field.ident.as_ref()?;
        let key = format!("{}.{}", &prefix, field_name);
        let lit = LitStr::new(&key, field_name.span());
        Some(quote! { #lit })
    });

    let struct_name = &item_struct.ident;
    let expanded = quote! {
        
        #item_struct

        impl #struct_name {
            pub fn property_keys() -> Vec<&'static str> {
                vec![#(#property_keys),*]
            }
        }
    };

    println!("aaaaaaexpanded: {:?}", expanded.to_string());
    Ok(expanded.into())
}
