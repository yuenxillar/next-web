use from_attr::FromAttr;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use std::convert::TryFrom;
use syn::{parse_macro_input, spanned::Spanned, ItemFn, LitStr};
use syn::{FnArg, Ident};

use crate::singleton::find::impl_find_attribute;

use super::attrs::request_mapping_attr::RequestMappingAttr;

pub fn with_method(method: Option<Method>, attr: TokenStream, input: TokenStream) -> TokenStream {
    let item_fn = parse_macro_input!(input as ItemFn);

    let vis = &item_fn.vis;
    let sig = &item_fn.sig;
    let name = &sig.ident;

    match vis {
        syn::Visibility::Inherited => {
            return syn::Error::new(vis.span(), "Visibility of function must be public")
                .to_compile_error()
                .into()
        }
        _ => {}
    };

    if sig.asyncness.is_none() {
        return syn::Error::new(sig.span(), "Function must be declared as async")
            .to_compile_error()
            .into();
    }

    if matches!(sig.output, syn::ReturnType::Default) {
        return syn::Error::new_spanned(
            item_fn,
            "Function has no return type. Cannot be used as handler",
        )
        .to_compile_error()
        .into();
    }

    let inputs = &sig.inputs;
    if inputs.iter().any(|val| match val {
        FnArg::Receiver(_) => true,
        _ => false,
    }) {
        return syn::Error::new(inputs.span(), "The function must not have a receiver, self")
            .to_compile_error()
            .into();
    }

    let RequestMappingAttr {
        method_,
        path,
        headers,
        consumes,
        produces,
    } = match RequestMappingAttr::from_tokens(attr.into()) {
        Ok(attr) => attr,
        Err(error) => return error.to_compile_error().into(),
    };

    let ast = match impl_find_attribute(&mut item_fn.clone(), produces) {
        Ok(stream) => stream,
        Err(e) => return e.to_compile_error().into(),
    };

    let method = match method {
        Some(method) => {
            // There should be mutual exclusion here
            if method_.is_some() {
                return syn::Error::new_spanned(
                    method_,
                    "Method should not be added to parameters other than RequestMapping",
                )
                .to_compile_error()
                .into();
            }
            method.to_ident()
        }
        None => {
            if let Some(method) = method_ {
                match Method::from_litstr(&method) {
                    Ok(mehtod) => mehtod.to_ident(),
                    Err(e) => return  e.to_compile_error().into()
                }
            }else {
                return syn::Error::new_spanned(
                    path,
                    "Method attribute is required when method parameter is not provided",
                )
                .to_compile_error()
                .into();
            }
        }
    };

    let doc_attributes: Vec<syn::Attribute> = item_fn
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .cloned()
        .collect();

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

    stream.into()
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

            pub fn as_lowercase_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => stringify!($lower),)+
                }
            }

            fn parse(method: &str) -> Result<Self, String> {
                match method {
                    $(stringify!($upper) => Ok(Self::$variant),)+
                    _ => Err(format!("HTTP method must be uppercase: `{}`", method)),
                }
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
