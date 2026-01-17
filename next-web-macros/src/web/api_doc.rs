use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{parse::Parser, punctuated::Punctuated, spanned::Spanned, Ident, Token};

use crate::{util::logic::Logic, web::routing::Method};

pub(crate) fn impl_macro_api_doc(attr: TokenStream, item: TokenStream) -> TokenStream {
    let expand = TokenStream2::from(attr);
    let item_fn = syn::parse_macro_input!(item as syn::ItemFn);

    // no check
    Logic::generate(|| {
        // Get Method and Path
        let (http_method, path) = match item_fn.attrs.iter().find(|attr| {
            match attr.path().get_ident() {
                Some(ident) => Method::values().iter().any(|mapping| ident == mapping),
                None => false,
            }
        }) {
            Some(attr) => {
                let span = attr.path().span();

                let ident = match attr.path().get_ident() {
                    Some(ident) => ident.to_string(),
                    None => return Err(syn::Error::new(span.clone(), "Invalid attribute path")),
                };
                let mapping = ident.trim_end();

                let method = match mapping
                    .split_once("_")
                    .map(|(s, _)| s.trim())
                    .map(|s| s.to_lowercase())
                    .map(|s| match s.as_str() {
                        "any" => 1,
                        "request" => 2,
                        _ => 0,
                    })
                    .unwrap_or(0)
                {
                    1 => {
                        Some(quote! { method(get, post, put, delete, options, head, patch, trace)})
                    }
                    2 => None,
                    _ => {
                        let method = match Method::parse_mapping(mapping) {
                            #[rustfmt::skip]
                            Ok(method) => Ident::new(method.as_lowercase_str(), span.clone()).to_token_stream(),
                            Err(err) => return Err(syn::Error::new(span.clone(), err)),
                        };

                        Some(method)
                    }
                };

                let list = match attr.meta.require_list() {
                    Ok(list) => list,
                    Err(err) => return Err(err),
                };

                let parser = Punctuated::<syn::MetaNameValue, Token![,]>::parse_terminated;
                let name_values = match parser.parse2(list.tokens.clone()) {
                    Ok(name_values) => name_values,
                    Err(err) => return Err(err),
                };

                let http_method = match method {
                    Some(method) => method,
                    None => {
                        match name_values
                            .iter()
                            .find(|item| item.path.is_ident("method"))
                            .map(|item| {
                                if let syn::Expr::Lit(expr) = &item.value {
                                    if let syn::Lit::Str(lit) = &expr.lit {
                                        let value = lit.value();
                                        if !value.is_empty() {
                                            return Some(value);
                                        }
                                    }
                                }

                                None
                            })
                            .unwrap_or_default()
                        {
                            Some(method) => Ident::new(&method.to_lowercase(), Span::call_site())
                                .to_token_stream(),
                            None => {
                                return Err(syn::Error::new(
                                    list.span(),
                                    "Missing HTTP method: `method`",
                                ))
                            }
                        }
                    }
                };

                let path = match name_values
                    .into_iter()
                    .find(|item| item.path.is_ident("path"))
                    .map(|item| {
                        if let syn::Expr::Lit(expr) = &item.value {
                            if let syn::Lit::Str(lit) = &expr.lit {
                                if !lit.value().is_empty() {
                                    return Some(item.clone());
                                }
                            }
                        }

                        None
                    })
                    .unwrap_or_default()
                {
                    Some(path) => path,
                    None => return Err(syn::Error::new(span, "Path attribute is required")),
                };

                (http_method, path)
            }
            None => return Ok(quote! {}),
        };

        let expanded = quote! {

            #[::utoipa::path(
                #http_method,
                #path,
                #expand
            )]
            #item_fn

        };

        // println!("expanded: {}", expanded.to_string());
        Ok(expanded)
    })
}
