use from_attr::FromAttr;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use std::convert::TryFrom;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{parse_macro_input, spanned::Spanned, ItemFn, LitStr};
use syn::{parse_quote, Expr, FnArg, Ident, Item, ItemImpl, Lit, Meta, Type};

use crate::util::field_type::FieldType;
use crate::util::logic::Logic;

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
    Logic::valid_method_handler(&item_fn)?;

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
    Logic::add_args(&mut item_fn, headers);

    let block = match generate_block(&mut item_fn, consume, produce) {
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

    // Build http method handler configurer
    // let http_method_handler_configurer = quote! {
    //     ::next_web_dev::configurer::http_method_handler_configurer::HttpMethodHandlerConfigurer::default()
    //     .with_idempotency_configurer();
    // };
    // let conditioned = quote! { true };

    let doc_attributes: Vec<syn::Attribute> = item_fn
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .cloned()
        .collect();

    let vis = &item_fn.vis;
    let name = &item_fn.sig.ident;

    let stream = quote! {
        #(#doc_attributes)*
        #[allow(non_camel_case_types)]
        #vis struct #name;

        impl ::next_web_dev::autoregister::handler_autoregister::HttpHandlerAutoRegister for #name {
            fn register<'a>(& self,
                __router:           ::next_web_dev::Router,
                __context:  &'a mut ::next_web_dev::configurer::http_method_handler_configurer::RouterContext
            ) -> ::next_web_dev::Router {
                #block

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

fn generate_block(
    item: &mut ItemFn,

    consume: Option<LitStr>,
    produce: Option<LitStr>,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let mut variables = Vec::new();

    for (index, fn_arg) in item.sig.inputs.iter_mut().enumerate() {
        match fn_arg {
            FnArg::Receiver(_) => {}
            FnArg::Typed(pat_type) => {
                match pat_type.ty.as_mut() {
                    syn::Type::Path(type_path) => {
                        if !pat_type
                            .attrs
                            .iter()
                            .any(|attr| attr.path().is_ident("find"))
                        {
                            continue;
                        }

                        let is_single = type_path
                            .path
                            .segments
                            .iter()
                            .any(|seg| seg.ident.to_string().contains("FindSingleton"));

                        for seg in type_path.path.segments.iter_mut() {
                            if seg.ident.to_string().contains("FindSingleton")
                                && match &seg.arguments {
                                    syn::PathArguments::AngleBracketed(_) => true,
                                    _ => false,
                                }
                            {
                                match &mut seg.arguments {
                                    syn::PathArguments::AngleBracketed(angle_bracketed) => {
                                        let mut flag = false;
                                        if let Some(syn::GenericArgument::Type(typ)) =
                                            angle_bracketed.args.first()
                                        {
                                            if let Type::Path(type_path) = typ {
                                                let original_variable = match pat_type.pat.as_ref() {
                                                    syn::Pat::Ident(pat_ident) => {
                                                        pat_ident.ident.clone()
                                                    }
                                                    syn::Pat::TupleStruct(pat_tuple_struct) => {
                                                        if let Some(syn::Pat::Ident(pat_ident)) =
                                                            pat_tuple_struct.elems.first()
                                                        {
                                                            pat_ident.ident.clone()
                                                        } else {
                                                            return Err(syn::Error::new(
                                                                pat_type.pat.span(),
                                                                 "The pattern must be an identifier or a tuple struct with one identifier"));
                                                        }
                                                    }
                                                    _ => return Err(syn::Error::new(

                                                                 pat_type.pat.span(),
                                                                 "The pattern must be an identifier or a tuple struct with one identifier",
                                                    ))
                                                };

                                                let arg = Type::Path(type_path.clone());

                                                let variable = Ident::new(
                                                    &format!("__my_service{}", index),
                                                    Span::call_site(),
                                                );
                                                let single_name = Ident::new(&crate::util::name::field_name_to_singleton_name(&original_variable.to_string()), Span::call_site());
                                                let stream = quote! {
                                                    let #original_variable = #variable.get_single_with_name::<#arg>(stringify!(#single_name)).await;
                                                };

                                                variables.push(stream);

                                                flag = true;
                                            }
                                        }

                                        if flag {
                                            angle_bracketed.args.clear();
                                            angle_bracketed.args.push(syn::GenericArgument::Type(
                                                    parse_quote!(::next_web_core::state::application_state::ApplicationState)
                                            ));

                                            let pat = syn::Pat::Ident(syn::PatIdent {
                                                attrs: vec![],
                                                by_ref: None,
                                                mutability: None,
                                                ident: Ident::new(
                                                    format!("__my_service{}", index).as_str(),
                                                    Span::call_site(),
                                                ),
                                                subpat: None,
                                            });
                                            pat_type.pat = Box::new(pat);
                                        }
                                    }

                                    _ => {}
                                }

                                pat_type.attrs.retain(|attr| !attr.path().is_ident("find"));
                            }
                        }

                        if is_single {
                            let arg: syn::Type = parse_quote!(
                                ::next_web_core::state::application_state::ApplicationState
                            );
                            let path: syn::Path =
                                parse_quote!(::next_web_dev::extract::Extension<#arg>);
                            type_path.path = path;
                        }
                    }
                    _ => {}
                };
            }
        }
    }

    let _return = item.block.stmts.iter().any(|stmt| match stmt {
        syn::Stmt::Expr(expr, _semi) => match expr {
            syn::Expr::Return(_) => true,
            _ => false,
        },
        _ => false,
    });

    let async_: proc_macro2::TokenStream = if _return {
        parse_quote!(async)
    } else {
        parse_quote!()
    };

    let await_: proc_macro2::TokenStream = if _return {
        parse_quote!(.await)
    } else {
        parse_quote!()
    };

    let modify_body = match produce {
        Some(produces) => quote! {
            resp.headers_mut()
            .insert(
                ::next_web_dev::http::header::CONTENT_TYPE,
                ::next_web_dev::http::HeaderValue::from_static(#produces))
            .unwrap();
        },
        None => quote! {},
    };

    let verify_content_type = match consume {
        Some(consumes) => {
            if !consumes.value().trim().is_empty() {
                // Add corresponding parameters to the function parameter list
                Logic::add_args(
                    item,
                    [
                        quote! {
                        ::next_web_dev::extract::typed_header::TypedHeader(__verify_content_type) : ::next_web_dev::extract::typed_header::TypedHeader<::next_web_dev::headers::ContentType>
                    }].into_iter(),
                );

                quote! {
                    if !__verify_content_type.to_string().contains(#consumes) {
                        return (::next_web_dev::http::StatusCode::UNSUPPORTED_MEDIA_TYPE, "Unsupported Media Type").into_response();
                    }
                }
            } else {
                quote! {}
            }
        }
        None => quote! {},
    };

    // Unified return type
    let unified_return_type = match syn::parse2::<syn::ReturnType>(
        quote! { -> impl ::next_web_dev::response::IntoResponse},
    ) {
        Ok(return_type) => return_type,
        Err(error) => return Err(error),
    };

    let return_type = if let syn::ReturnType::Type(_, typ) = &item.sig.output {
        if FieldType::is_result(typ.as_ref()) {
            quote! {: #typ }
        } else {
            quote! {}
        }
    } else {
        quote! {}
    };

    item.sig.output = unified_return_type;

    let sig = &item.sig;
    let block = &item.block.stmts;
    let vis = &item.vis;

    let token_stream = quote! {
        #vis #sig
        {
            #verify_content_type

            let result #return_type  =
            #async_
            {
                #(#variables)*

                #(#block)*

            }
            #await_ ;

            let mut resp = result.into_response();

            #modify_body

            resp
        }
    };

    // println!("token_stream: \n{}", token_stream.to_string());

    Ok(token_stream.into())
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
