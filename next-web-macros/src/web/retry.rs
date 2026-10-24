use from_attr::FromAttr;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{spanned::Spanned, Expr, ItemFn, ReturnType, Type};

use crate::{
    util::param_info::{extract_param_info, ParamInfo},
    web::attrs::retry_attr::RetryAttr,
};

pub fn impl_macro_retry(attr: TokenStream, item: ItemFn) -> TokenStream {
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

    // Validate `multiplier`: it must be a positive integer.
    if let Some(Expr::Lit(expr_lit)) = &multiplier {
        if let syn::Lit::Int(lit_int) = &expr_lit.lit {
            if let Ok(num) = lit_int.base10_parse::<u8>() {
                if num < 1 {
                    return syn::Error::new(
                        expr_lit.span(),
                        "multiplier must be greater than or equal to 1",
                    )
                    .to_compile_error()
                    .into();
                }
            }
        }
    }

    let fn_name = &item.sig.ident;
    let fn_block = &item.block;
    let is_async = item.sig.asyncness.is_some();
    let fn_inputs = &item.sig.inputs;
    let fn_output = &item.sig.output;

    // The macro can only be applied to functions returning `Result`.
    let is_result = match fn_output {
        ReturnType::Default => false,
        ReturnType::Type(_, ty) => matches!(
            &**ty,
            Type::Path(tp) if tp.path.segments.last()
                .map_or(false, |seg| seg.ident == "Result" || seg.ident == "StdResult")
        ),
    };

    if !is_result {
        return syn::Error::new_spanned(
            &item.sig,
            "The retry macro can only be applied to functions that return Result",
        )
        .to_compile_error()
        .into();
    }

    // Collect parameter info and clone non-reference parameters on each attempt.
    let params: Vec<ParamInfo> = extract_param_info(&item);

    let clones = params
        .iter()
        .filter(|p| !p.is_reference)
        .map(|param| {
            let name = syn::Ident::new(&param.name, Span::call_site());
            quote! { let #name = #name.clone(); }
        })
        .collect::<Vec<_>>();

    // IMPORTANT: generate the backoff snippet before `retry_for`,
    // so that `#backoff` inside `retry_for` refers to the full call
    // (`test_backoff(&error);`) rather than the bare identifier.
    let backoff = match backoff {
        Some(backoff) => quote! { #backoff(&error); },
        None => quote! {},
    };

    // Build the early-return branch for errors that should not be retried.
    let retry_for = if retry_for.is_empty() {
        quote! {}
    } else {
        quote! {
            if !match error {
                #(#retry_for)|* => true,
                _ => false,
            } {
                #backoff
                return Err(error);
            }
        }
    };

    // Delay multiplier applied after each failed attempt.
    let multiplier = match multiplier {
        Some(multiplier) => quote! { delay = delay * #multiplier; },
        None => quote! {},
    };

    // Generate the retry loop, with different sleep strategies for async and sync.
    let retry_logic = if is_async {
        quote! {
            async fn #fn_name(#fn_inputs) #fn_output {
                let max_attempts: u8 = #max_attempts;
                let mut delay: u64 = #delay;
                let mut attempt_count: u8 = 1;

                loop {
                    #(#clones)*

                    let result = async move #fn_block.await;
                    match result {
                        Ok(value) => return Ok(value),
                        Err(error) => {
                            #retry_for

                            if attempt_count >= max_attempts {
                                #backoff
                                return Err(error);
                            }

                            ::tokio::time::sleep(
                                ::std::time::Duration::from_millis(delay)
                            ).await;

                            #multiplier
                            attempt_count += 1;
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
                let mut attempt_count: u8 = 1;

                loop {
                    #(#clones)*

                    let result = #fn_block;
                    match result {
                        Ok(value) => return Ok(value),
                        Err(error) => {
                            #retry_for

                            if attempt_count >= max_attempts {
                                #backoff
                                return Err(error);
                            }

                            ::std::thread::sleep(
                                ::std::time::Duration::from_millis(delay)
                            );

                            #multiplier
                            attempt_count += 1;
                        }
                    }
                }
            }
        }
    };

    // println!("token stream: {}", retry_logic.to_string());

    retry_logic.into()
}
