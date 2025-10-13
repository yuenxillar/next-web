use from_attr::FromAttr;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::ItemFn;

use crate::{util::logic::Logic, web::attrs::idempotency_attr::IdempotencyAttr};
pub(crate) fn impl_macro_idempotency(attr: TokenStream, item_fn: ItemFn) -> TokenStream {
    Logic::generate(move || {
        let IdempotencyAttr {
            key,
            cache_key_prefix,
            ttl,
        } = match IdempotencyAttr::from_tokens(attr.clone().into()) {
            Ok(attr) => attr,
            Err(e) => return Err(e),
        };


        
        Ok(quote! {})
    })
}
