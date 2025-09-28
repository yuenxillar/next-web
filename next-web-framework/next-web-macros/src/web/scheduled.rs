use from_attr::FromAttr;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;

use super::attrs::scheduled_attr::ScheduledAttr;

macro_rules! quote_some {
    ($opt:expr) => {
        match $opt {
            Some(ref x) => quote::quote! { Some(#x) },
            None => quote::quote! { None },
        }
    };
}

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
        timezone,
        time_unit,

        one_shot,
    } = match ScheduledAttr::from_tokens(attr.into()) {
        Ok(attr) => attr,
        Err(error) => return error.to_compile_error().into(),
    };

    let args: Vec<Box<syn::Pat>> = sig
        .inputs
        .iter()
        .map(|input| match input {
            syn::FnArg::Receiver(_receiver) => {
                panic!("Self should not exist in function parameters")
            }
            syn::FnArg::Typed(pat_type) => pat_type.pat.clone(),
        })
        .collect();

    let variables: Vec<TokenStream2> = sig
        .inputs
        .iter()
        .map(|input| match input {
            syn::FnArg::Receiver(_receiver) => panic!("Self should not exist in function parameters"),
            syn::FnArg::Typed(pat_type) => {
                let variable_type = &pat_type.ty;
                let variable_name = &pat_type.pat;

                let find = pat_type.attrs.iter().any(|attr| attr.path().is_ident("find"));
                if !find {
                    quote! {
                        let #variable_name = __ctx.resolve_with_default_name::<#variable_type>();
                    }
                }else {
                    quote! {
                        let #variable_name = __ctx.resolve_with_name::<#variable_type>(stringify!(#variable_name));
                    }
                }
            }
        })
        .collect();

    let cloneds = args
        .iter()
        .map(|arg| quote! { let #arg = #arg.clone(); })
        .collect::<Vec<_>>();
    let mark = if is_async {
        quote! {Async}
    } else {
        quote! {Sync}
    };

    let schedule = if one_shot {
        if let Some(initial_delay) = initial_delay.as_ref() {
            let num: u64 = initial_delay.base10_parse().unwrap();
            if num <= 0 {
                return syn::Error::new(
                    initial_delay.span(),
                    "initial_delay must be greater than 0",
                )
                .to_compile_error()
                .into();
            }
        } else {
            return syn::Error::new(
                initial_delay
                    .map(|lit| lit.span())
                    .unwrap_or(Span::call_site()),
                "one_shot must be set with initial_delay",
            )
            .to_compile_error()
            .into();
        }

        let timezone = quote_some!(timezone);
        let time_unit = quote_some!(time_unit);

        quote! {::next_web_dev::scheduler::schedule_type::ScheduleType::OneShot(
            ::next_web_dev::scheduler::schedule_type::WithArgs {
                    initial_delay:  Some(#initial_delay),
                    timezone:       #timezone,
                    time_unit:      #time_unit,
                    ..Default::default()
            }
        )}
    } else {
        if let Some(cron) = cron {
            let timezone = quote_some!(timezone);

            quote! {::next_web_dev::scheduler::schedule_type::ScheduleType::Cron(
                ::next_web_dev::scheduler::schedule_type::WithArgs {
                    cron:           Some(#cron),
                    timezone:       #timezone,
                    ..Default::default()
                }
            )}
        } else {
            let time_unit = quote_some!(time_unit);

            quote! {::next_web_dev::scheduler::schedule_type::ScheduleType::FixedRate(
                ::next_web_dev::scheduler::schedule_type::WithArgs {
                        fixed_rate: Some(#fixed_rate),
                        time_unit: #time_unit,
                        ..Default::default()
                }
            )}
        }
    };

    let run = if is_async {
        quote! {
            ::std::boxed::Box::new(move || { #(#cloneds)* ::std::boxed::Box::pin(#name(#(#args),*)) })
        }
    } else {
        quote! {::std::boxed::Box::new(#name(#(#args),*))}
    };

    let doc_attributes: Vec<syn::Attribute> = item_fn
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .cloned()
        .collect();

    let expanded = quote! {
        #(#doc_attributes)*
        #[allow(non_camel_case_types)]
        #vis struct #name;

        impl ::next_web_dev::autoregister::scheduler_autoregister::SchedulerAutoRegister for #name {
            fn register(& self, __ctx: &mut ::next_web_dev::ApplicationContext) -> ::next_web_dev::autoregister::scheduler_autoregister::AnJob {

                #item_fn

                #( #variables )*

                ::next_web_dev::autoregister::scheduler_autoregister::AnJob::#mark ( (#schedule , #run) )
            }
        }

        ::next_web_dev::submit_scheduler!(#name);
    };

    // println!("token_stream: {}", expanded.to_string());

    expanded.into()
}
