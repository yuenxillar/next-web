use from_attr::FromAttr;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{spanned::Spanned, Expr, Ident, ItemFn, Lit, LitStr};

use crate::{
    util::{id::unique_id, logic::Logic},
    web::attrs::idempotency_attr::IdempotencyAttr,
};
pub(crate) fn impl_macro_idempotency(attr: TokenStream, mut item_fn: ItemFn) -> TokenStream {
    Logic::generate(|| {
        Logic::valid_method_handler(&item_fn)?;

        // Only allow Post requests to pass
        if !&item_fn.attrs.iter().all(|attri| {
            attri.meta.path().is_ident("PostMapping")
            ||
            if attri.meta.path().is_ident("RequestMapping") {
                match &attri.meta {
                    syn::Meta::List(meta_list) => {
                        meta_list.parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::token::Comma>::parse_terminated).map(
                            |value| value.iter().any(|
                                item| {
                                    if let syn::Meta::NameValue(name) = item {
                                        if name.path.is_ident("method") {
                                            if let Expr::Lit(expr_lit) = & name.value {
                                                if let Lit::Str(lit_str) = & expr_lit.lit {
                                                    return lit_str.value().trim().to_uppercase() == "POST";
                                                }
                                            }
                                        }
                                    }
                                    false
                                })
                        ).unwrap_or_default()
                    },
                    _ => false
                }
            }else { false }
        }) {
            return Err(syn::Error::new(item_fn.span(), "Idempotency macro Only allow Post requests to pass."));
        }

        let IdempotencyAttr {
            name,
            key,
            cache_key_prefix,
            ttl,
        } = match IdempotencyAttr::from_tokens(attr.clone().into()) {
            Ok(attr) => attr,
            Err(e) => return Err(e),
        };

        // Args
        // HeaderMap
        let headers = quote! { headers: ::next_web_dev::http::HeaderMap };

        let extension = Ident::new(
            &format!("__my_extension_{}", unique_id()),
            Span::call_site(),
        );
        let idempotency_store = quote! {
            ::next_web_dev::extract::Extension( #extension ) :
            ::next_web_dev::extract::Extension< ::next_web_dev::state::application_state::ApplicationState >
        };

        let key = key
            .filter(|k| !k.value().trim().is_empty())
            .unwrap_or(LitStr::new("Idempotency-Key", Span::call_site()));
        let name = name
            .filter(|k| !k.value().trim().is_empty())
            .unwrap_or(LitStr::new("memoryIdempotencyStore", Span::call_site()));
        let ttl = ttl
            .map(|ttl| quote! { Some(#ttl) })
            .unwrap_or(quote! { Some(7) });
        let cache_key = cache_key_prefix
            .filter(|k| !k.value().trim().is_empty())
            .map(|k| {
                quote! { & format!("{}:{}", #k, __idempotency_key) }
            })
            .unwrap_or(quote! { __idempotency_key });

        let block = quote! {
            let __idempotency_key = match headers.get(#key).and_then(|v| v.to_str().ok()) {
                Some(key) => key,
                None => {
                    return (::next_web_dev::http::StatusCode::BAD_REQUEST, "Bad Request")
                        .into_response()
                }
            };

            let __idempotency_store = #extension.context()
                .read()
                .await
                .get_single_with_name::<::std::sync::Arc<dyn ::next_web_dev::traits::store::idempotency_store::IdempotencyStore<Value = ()>> >( #name )
                .clone();

            match __idempotency_store.check_and_store(#cache_key, Some(()), #ttl).await {
                Ok(_value) => {
                    if let Some(value) = _value {
                        return __idempotency_store.to_error_response(None);
                    }
                },
                Err(_error) => return __idempotency_store.to_error_response(Some(_error.to_string()))
            };

        };

        Logic::add_args(&mut item_fn, [headers, idempotency_store].into_iter());
        Logic::add_block(&mut item_fn, block);

        // println!("toekn_stream:  {}", item_fn.to_token_stream().to_string());

        Ok(quote! { #item_fn })
    })
}
