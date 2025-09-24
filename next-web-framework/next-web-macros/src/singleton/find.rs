use proc_macro2::Span;
use quote::quote;
use syn::{parse_quote, spanned::Spanned, Error, FnArg, Ident, ItemFn, LitStr, Type};

pub(crate) fn impl_find_attribute(
    item: &mut ItemFn,

    produces: Option<LitStr>,
) -> Result<proc_macro2::TokenStream, Error> {
    let vis = &item.vis;
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
                                                            return Err(Error::new(
                                                                pat_type.pat.span(),
                                                                 "The pattern must be an identifier or a tuple struct with one identifier"));
                                                        }
                                                    }
                                                    _ => return Err(Error::new(

                                                                 pat_type.pat.span(),
                                                                 "The pattern must be an identifier or a tuple struct with one identifier",
                                                    ))
                                                };

                                                let arg = Type::Path(type_path.clone());

                                                let variable = Ident::new(
                                                    &format!("_my_service{}", index),
                                                    Span::call_site(),
                                                );
                                                let single_name = Ident::new(&crate::util::single::field_name_to_singleton_name(&original_variable.to_string()), Span::call_site());
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
                                                    format!("_my_service{}", index).as_str(),
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
                            let path: syn::Path = parse_quote!(::next_web_dev::Extension<#arg>);
                            type_path.path = path;
                        }
                    }
                    _ => {}
                };
            }
        }
    }

    let sig = &item.sig;
    let block = &item.block.stmts;

    let _return = block.iter().any(|stmt| match stmt {
        syn::Stmt::Expr(expr, _semi) => match expr {
            syn::Expr::Return(_) => true,
            _ => false,
        },
        _ => false,
    });

    let async_: proc_macro2::TokenStream = if  _return{
        parse_quote!(async)
    } else {
        parse_quote!()
    };

    let await_: proc_macro2::TokenStream = if _return {
        parse_quote!(.await)
    } else {
        parse_quote!()
    };

    let modify_body = match produces {
        Some(produces) => quote! {
            body.headers_mut()
            .insert(
                ::next_web_dev::http::header::CONTENT_TYPE,
                ::next_web_dev::http::HeaderValue::from_static(#produces))
            .unwrap();
        },
        None => quote! {},
    };

    let token_stream = quote! {
        #vis #sig
        {
            #[allow(unused_mut)]
            let mut body = 
            #async_ {
                #(#variables)*

                #(#block)*

            }
            #await_
            .into_response();

            #modify_body
    
            body
        }
    };

    // println!("token_stream: \n{}", token_stream.to_string());

    Ok(token_stream.into())
}
