use from_attr::FromAttr;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use std::convert::TryFrom;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{parse_macro_input, spanned::Spanned, ItemFn, LitStr};
use syn::{Expr, FnArg, Ident, Item, ItemImpl, Lit, Meta};

use crate::singleton::find::impl_find_attribute;
use crate::util::attributes::add_args;

use super::attrs::request_mapping_attr::RequestMappingAttr;

pub fn with_method(method: Option<Method>, attr: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    let result = match item {
        Item::Fn(item_fn) => impl_item_fn(method, item_fn, attr),
        Item::Impl(item_impl) => impl_item_impl(method, item_impl, attr),
        _ => {
            return syn::Error::new(item.span(), "Only supports functions and impacts")
                .to_compile_error()
                .into()
        }
    };
    result.unwrap_or_else(|e| e.to_compile_error()).into()
}

fn impl_item_fn(
    method: Option<Method>,
    mut item_fn: ItemFn,
    attr: TokenStream,
) -> Result<TokenStream2, syn::Error> {

    if item_fn.sig.asyncness.is_none() {
        return Err(syn::Error::new(
            item_fn.sig.span(),
            "Function must be declared as async",
        ));
    }

    if matches!(item_fn.sig.output, syn::ReturnType::Default) {
        return Err(syn::Error::new_spanned(
            item_fn,
            "Function has no return type. Cannot be used as handler",
        ));
    }

    let inputs = &item_fn.sig.inputs;
    if inputs.iter().any(|val| match val {
        FnArg::Receiver(_) => true,
        _ => false,
    }) {
        return Err(syn::Error::new(
            inputs.span(),
            "The function must not have a receiver, self",
        ));
    }

    let RequestMappingAttr {
        method_,
        path,
        headers,
        consume,
        produce,
    } = match RequestMappingAttr::from_tokens(attr.into()) {
        Ok(attr) => attr,
        Err(error) => return Err(error),
    };


    let headers = headers.into_iter()
    .filter(|header| !header.value().trim().is_empty() )
    .enumerate()
        .map(|(index, header)| {
            let header = Ident::new(& header.value().trim(), Span::call_site());
            let var_name = Ident::new(& format!("__required_header_{}", index), Span::call_site());
            quote! {
                #var_name : ::next_web_dev::extract::RequiredHeader<::next_web_dev::header_names::#header>
            }
        });

    // Add the required parameters to the function's argument list
    add_args(&mut item_fn, headers);

    let ast = match impl_find_attribute(&mut item_fn, consume, produce) {
        Ok(stream) => stream,
        Err(e) => return Err(e),
    };

    let method = match method {
        Some(method) => {
            // There should be mutual exclusion here
            if method_.is_some() {
                return Err(syn::Error::new_spanned(
                    method_,
                    "Method should not be added to parameters other than RequestMapping",
                ));
            }
            method.to_ident()
        }
        None => {
            // RequestMapping
            if let Some(method) = method_ {
                match Method::from_litstr(&method) {
                    Ok(mehtod) => mehtod.to_ident(),
                    Err(e) => return Err(e),
                }
            } else {
                return Err(syn::Error::new_spanned(
                    path,
                    "`method` attribute is required when method parameter is not provided",
                ));
            }
        }
    };

    let doc_attributes: Vec<syn::Attribute> = item_fn
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .cloned()
        .collect();

    let vis = &item_fn.vis;
    let name     = &item_fn.sig.ident;
    
    let stream = quote! {
        #(#doc_attributes)*
        #[allow(non_camel_case_types)]
        #vis struct #name;

        impl ::next_web_dev::autoregister::handler_autoregister::HttpHandlerAutoRegister for #name {
            fn register(& self, __router: ::next_web_dev::Router) -> ::next_web_dev::Router {
                #ast

                __router.route(#path, ::next_web_dev::routing::#method(#name))
            }
        }

        ::next_web_dev::submit_handler!(#name);
    };

    // println!("token stream: {}", stream.to_string());

    Ok(stream)
}

fn impl_item_impl(
    method: Option<Method>,
    mut item_impl: ItemImpl,
    attr: TokenStream,
) -> Result<TokenStream2, syn::Error> {
    if let Some(method) = method {
        return Err(syn::Error::new(
            method.span(),
            format!(
                "`{}` cannot be added to an implementation block",
                method.to_string()
            ),
        ));
    }

    #[allow(unused_variables)]
    let RequestMappingAttr {
        method_,
        path,
        headers,
        consume,
        produce,
    } = match RequestMappingAttr::from_tokens(attr.into()) {
        Ok(attr) => attr,
        Err(error) => return Err(error),
    };

    if let Some(method) = method_ {
        return Err(syn::Error::new(
            method.span(),
            "`method` attribute cannot be added to an implementation block",
        ));
    }

    if item_impl.trait_.is_some() {
        return Err(syn::Error::new(
            item_impl.span(),
            "Implementation block cannot have a trait",
        ));
    }

    let path = match path {
        Expr::Lit(expr_lit) => {
            if let Lit::Str(lit_str) = expr_lit.lit {
                lit_str
            } else {
                return Err(syn::Error::new(
                    expr_lit.span(),
                    "Path attribute must be a string literal",
                ));
            }
        }
        _ => {
            return Err(syn::Error::new_spanned(
                path,
                "Path attribute must be a string literal",
            ))
        }
    }
    .value();

    let mut indexs = item_impl
        .items
        .iter_mut()
        .filter_map(|item| match item {
            syn::ImplItem::Fn(impl_item_fn) => Some(impl_item_fn),
            _ => None,
        })
        .filter_map(|impl_item_fn| {
            let mapping = impl_item_fn.attrs.iter_mut().find(|attr| {
                Method::values()
                    .iter()
                    .any(|&method| attr.path().is_ident(method))
            });

            if impl_item_fn.sig.asyncness.is_some() {
                match mapping {
                    Some(m) => Some(m),
                    None => None,
                }
            } else {
                None
            }
        })
        .enumerate()
        .filter_map(|(index, attri)| {
            let meta_list = match &mut attri.meta {
                Meta::List(list) => list,
                _ => return None,
            };

            let mut nested_meta: Punctuated<Meta, Comma> =
                match meta_list.parse_args_with(Punctuated::<Meta, Comma>::parse_terminated) {
                    Ok(nested_meta) => nested_meta,
                    Err(_) => return None,
                };

            for meta in nested_meta.iter_mut() {
                if let Meta::NameValue(name_value) = meta {
                    if name_value.path.is_ident("path") {
                        if let Expr::Lit(expr_lit) = &mut name_value.value {
                            if let Lit::Str(lit_str) = &mut expr_lit.lit {
                                *lit_str = LitStr::new(
                                    &format!("{}{}", path, lit_str.value()),
                                    lit_str.span(),
                                );
                                meta_list.tokens = nested_meta.to_token_stream();
                                return Some(index);
                            }
                        }
                    }
                }
            }
            None
        })
        .collect::<Vec<_>>();

    indexs.dedup();
    indexs.reverse();

    let functions = indexs
        .iter()
        .map(|index| item_impl.items.remove(*index))
        .collect::<Vec<_>>();
    let stream = quote! {
        #item_impl

        #(#functions)*
    };

    // println!("token stream: {}", stream.to_string());

    Ok(stream.into())
}

macro_rules! standard_method_type {
    (
        $($variant:ident, $upper:ident, $lower:ident,)+
    ) => {
        #[doc(hidden)]
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum Method {
            $(
                $variant,
            )+
        }

        impl Method {
            fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => stringify!($variant),)+
                }
            }

            fn as_lowercase_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => stringify!($lower),)+
                }
            }

            fn to_string(&self) -> String {
                match self {
                    $(Self::$variant => format!("{}{}", stringify!($variant),"Mapping"),)+
                }
            }

            fn parse(method: &str) -> Result<Self, String> {
                match method {
                    $(stringify!($upper) => Ok(Self::$variant),)+
                    _ => Err(format!("HTTP method must be uppercase: `{}`", method)),
                }
            }

            fn values() -> Vec<&'static str> {
                vec![
                    "GetMapping", "PostMapping", "PutMapping", "DeleteMapping", "PatchMapping", "RequestMapping", "AnyMapping"
                ]
            }

            #[allow(unused)]
            pub(crate) fn from_litstr(method: &LitStr) -> Result<Self, syn::Error> {
                let value = method.value();
                match () {
                    $(_ if value.eq_ignore_ascii_case(stringify!($lower)) => Ok(Self::$variant),)+
                    _ => Err(syn::Error::new(
                        method.span(),
                        format!("unknown HTTP method: {}", value),
                    )),
                }
            }
        }
    };
}

standard_method_type! {
    Get,       GET,     get,
    Post,      POST,    post,
    Put,       PUT,     put,
    Delete,    DELETE,  delete,
    Head,      HEAD,    head,
    Connect,   CONNECT, connect,
    Options,   OPTIONS, options,
    Trace,     TRACE,   trace,
    Patch,     PATCH,   patch,
    Any,       ANY,     any,
}

impl TryFrom<&syn::LitStr> for Method {
    type Error = syn::Error;

    fn try_from(value: &syn::LitStr) -> Result<Self, Self::Error> {
        Self::parse(value.value().as_str())
            .map_err(|message| syn::Error::new_spanned(value, message))
    }
}

impl Method {
    fn to_ident(&self) -> Ident {
        Ident::new(self.as_lowercase_str(), Span::call_site())
    }
}

impl ToTokens for Method {
    fn to_tokens(&self, stream: &mut TokenStream2) {
        let ident = Ident::new(self.as_str(), Span::call_site());
        stream.append(ident);
    }
}
