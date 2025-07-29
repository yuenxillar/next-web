use from_attr::FromAttr;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, Expr, ItemFn, ReturnType, Type};

use crate::{
    common::retry_attr::RetryAttr,
    utils::param_info::{extract_param_info, ParamInfo},
};

pub(crate) fn impl_macro_retry(attr: TokenStream, item: TokenStream) -> TokenStream {
    // 解析属性参数
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

    // 判断参数是否合法
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

    // 解析函数
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_block = &input_fn.block;
    let is_async = input_fn.sig.asyncness.is_some();
    let fn_inputs = &input_fn.sig.inputs;
    let fn_output = &input_fn.sig.output;

    // 获取参数的信息
    let params: Vec<ParamInfo> = extract_param_info(&input_fn);

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

    // 检查函数返回值是否为Result
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

    // 生成重试逻辑
    let retry_logic = if is_async {
        quote! {
            async fn #fn_name(#fn_inputs) #fn_output {
                let max_attempts: u8 = #max_attempts;
                let mut delay: u64 = #delay;

                let mut retry_count: u8 = 0;
                loop {
                    #(#clones)*

                    let result = async move #fn_block.await;
                    match result {
                        Ok(var) => return Ok(var),
                        Err(error) => {

                            #retry_for

                            if retry_count >= max_attempts {
                                #backoff
                                return Err(error);
                            }

                            retry_count += 1;

                            ::tokio::time::sleep(::std::time::Duration::from_millis(delay)).await;

                            #multiplier
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

                let mut retry_count: u8 = 0;

                loop {
                    #(#clones)*

                    let result = #fn_block;
                    match result {
                        Ok(var) => return Ok(var),
                        Err(error) => {

                            #retry_for

                           if retry_count >= max_attempts {
                                #backoff
                                return Err(error);
                            }

                            retry_count += 1;

                            ::std::thread::sleep(::std::time::Duration::from_millis(delay));

                            #multiplier
                        }
                    }
                }
            }
        }
    };

    // println!("retry_logic: {}", retry_logic.to_string());
    retry_logic.into()
}
