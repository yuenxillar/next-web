use from_attr::FromAttr;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{ spanned::Spanned, Expr, ItemFn, ReturnType, Type};

use crate::{
    common::retry_attr::RetryAttr,
    util::param_info::{extract_param_info, ParamInfo},
};

pub(crate) fn impl_macro_retry(attr: TokenStream, item: ItemFn) -> TokenStream {
    let RetryAttr {
        max_attempts,
        delay,
        backoff,
        retry_for,
        multiplier,
    } = match RetryAttr::from_tokens(attr.into()) {
        Ok(attr) => attr,
        Err(err) => return err.to_compile_error().into(),
    };

    // Determine whether the parameters are valid
    if let Some(multilier) = &multiplier {
        if let Expr::Lit(expr_lit) = multilier {
            if let syn::Lit::Int(lit_int) = &expr_lit.lit {
                if let Ok(num) = lit_int.token().to_string().parse::<u8>() {
                    if num < 1 {
                        return Err(syn::Error::new(
                            expr_lit.span(),
                            "max_attempts must be greater than or equal to 1",
                        ))
                        .unwrap_or_else(|e| e.to_compile_error())
                        .into();
                    }
                }
            }
        }
    }

    let fn_name = &item.sig.ident;
    let fn_block = &item.block;
    let is_async = item.sig.asyncness.is_some();
    let fn_inputs = &item.sig.inputs;
    let fn_output = &item.sig.output;

    //  Obtain information on parameters
    let params: Vec<ParamInfo> = extract_param_info(&item);

    let clones = params
        .iter()
        .filter(|p| !p.is_reference)
        .map(|param| {
            let name = syn::Ident::new(&param.name, Span::call_site().into());
            return quote! { let #name = #name.clone();};
        })
        .collect::<Vec<_>>();

    let backoff = match backoff {
        Some(backoff) => quote! { #backoff(& error);},
        None => quote! {},
    };

    let retry_for = if retry_for.len() > 0 {
        quote! {
           if !match error {
               #(#retry_for)|* => true,
               _ => false,
           } { #backoff return Err(error); }
        }
    } else {
        quote! {}
    };

    let multiplier = match multiplier {
        Some(multiplier) => quote! { delay = delay * #multiplier; },
        None => quote! {},
    };

    // Check if the return value of the function is Result
    if !match fn_output {
        ReturnType::Default => false,
        ReturnType::Type(_, ty) => {
            if let Type::Path(type_path) = &**ty {
                type_path
                    .path
                    .segments
                    .last()
                    .map_or(false, |seg| seg.ident == "Result")
            } else {
                false
            }
        }
    } {
        panic!("The retry macro can only be applied to functions that return Result");
    }

    // Generate the retry logic
    let retry_logic = if is_async {
        quote! {
            async fn #fn_name(#fn_inputs) #fn_output {
                let max_attempts: u8 = #max_attempts;
                let mut delay: u64 = #delay;

                let mut attempt_count : u8 = 0;
                loop {
                    #(#clones)*

                    let result = async move #fn_block.await;
                    match result {
                        Ok(var) => return Ok(var),
                        Err(error) => {
                            #retry_for

                            if attempt_count   >=  max_attempts - 1 {
                                #backoff
                                return Err(error);
                            }

                            ::tokio::time::sleep(::std::time::Duration::from_millis(delay)).await;

                            #multiplier
                            
                            attempt_count  += 1;
                        }
                    }
                }

            }
        }
    } else {
        quote! {
            fn #fn_name(#fn_inputs) #fn_output {

                let max_attempts: u8 = #max_attempts;
                let mut delay: u64 = #delay;

                let mut attempt_count : u8 = 0;
                loop {
                    #(#clones)*

                    let result = #fn_block;
                    match result {
                        Ok(var) => return Ok(var),
                        Err(error) => {
                            #retry_for

                           if attempt_count   >=  max_attempts - 1  {
                                #backoff
                                return Err(error);
                            }

                            ::std::thread::sleep(::std::time::Duration::from_millis(delay));

                            #multiplier

                            attempt_count  += 1;
                        }
                    }
                }
            }
        }
    };

    // println!("retry_logic: {}", retry_logic.to_string());
    retry_logic.into()
}
