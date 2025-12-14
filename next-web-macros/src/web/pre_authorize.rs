use from_attr::FromAttr;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{spanned::Spanned, ItemFn};

use crate::util::logic::Logic;

use super::attrs::pre_authorize_attr::PreAuthorizeAttr;

pub fn impl_macro_pre_authorize(attrs: TokenStream, item_fn: ItemFn) -> TokenStream {
    let expanded = Logic::generate(|| {
        let vis = &item_fn.vis;
        let sig = &item_fn.sig;
        let name = &sig.ident;

        if sig.asyncness.is_none() {
            return Err(syn::Error::new(
                sig.span(),
                "Function must be declared as async",
            ));
        }

        if matches!(sig.output, syn::ReturnType::Default) {
            return Err(syn::Error::new(
                sig.output.span(),
                "Function has no return type. Cannot be used as handler",
            ));
        }

        let PreAuthorizeAttr {
            role,
            permission,
            mode,
            ignore,
            basic,
        } = match PreAuthorizeAttr::from_tokens(attrs.clone().into()) {
            Ok(attr) => attr,
            Err(error) => return Err(error),
        };

        Ok(TokenStream2::new())
    });

    expanded
}
