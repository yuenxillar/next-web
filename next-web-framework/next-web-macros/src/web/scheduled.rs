use std::vec;

use from_attr::FromAttr;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use super::attrs::scheduled_attr::ScheduledAttr;

pub(crate) fn impl_macro_scheduled(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_fn = syn::parse_macro_input!(item as syn::ItemFn);

    let sig = &item_fn.sig;
    let vis = &item_fn.vis;
    let name = &sig.ident;

    let is_async = sig.asyncness.is_some();
    let ScheduledAttr {
        cron,
        fixed_rate,
        initial_delay,
    } = match ScheduledAttr::from_tokens(attr.into()) {
        Ok(attr) => attr,
        Err(error) => return error.to_compile_error().into(),
    };

    // sig.inputs.iter().all(|s| match s {
    //     syn::FnArg::Receiver(_receiver) => false,
    //     syn::FnArg::Typed(pat_type) => {

    //         match pat_type.ty {
    //             syn::Type::Path(type_path) => type_path.path.segments
    //             _ => todo!(),
    //         }
    //     }
    // });

    let doc_attributes: Vec<syn::Attribute> = item_fn
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .cloned()
        .collect();

    let args: Vec<Box<syn::Pat>> = sig
        .inputs
        .iter()
        .map(|input| match input {
            syn::FnArg::Receiver(_receiver) => todo!(),
            syn::FnArg::Typed(pat_type) => pat_type.pat.clone(),
        })
        .collect();

    let variables: Vec<TokenStream2> = sig
        .inputs
        .iter()
        .map(|input| match input {
            syn::FnArg::Receiver(_receiver) => todo!(),
            syn::FnArg::Typed(pat_type) => {
                let variable_type = &pat_type.ty;
                let variable_name = &pat_type.pat;
                quote! {
                    let #variable_name = __ctx.resolve_with_default_name::<#variable_type>();
                }
            }
        })
        .collect();

    let cloneds = args.iter().map(|arg| quote! { let #arg = #arg.clone(); }).collect::<Vec<_>>();
    let mark = if is_async {
        quote! {Async}
    } else {
        quote! {Sync}
    };
    let run = if is_async {
        quote! {
            ::std::boxed::Box::new(move || { #(#cloneds)* ::std::boxed::Box::pin(#name(#(#args),*)) })
        }
    } else {
        quote! {::std::boxed::Box::new(#name(#(#args),*))}
    };

    let stream = quote! {
        #(#doc_attributes)*
        #[allow(non_camel_case_types)]
        #vis struct #name;

        impl ::next_web_dev::autoregister::scheduler_autoregister::SchedulerAutoRegister for #name {
            fn register(& self, __ctx: &mut ::next_web_dev::ApplicationContext) -> ::next_web_dev::autoregister::scheduler_autoregister::AnJob {

                #item_fn

                #( #variables )*
                
                ::next_web_dev::autoregister::scheduler_autoregister::AnJob::#mark (#run)
            }
        }

        ::next_web_dev::submit_scheduler!(#name);
    };

    println!("token_stream: {}", stream.to_string());

    
    stream.into()
}
