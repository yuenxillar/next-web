use from_attr::FromAttr;
use proc_macro::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, ItemFn};

use crate::util::logic::Logic;

use super::attrs::pre_authorize_attr::PreAuthorizeAttr;

pub fn impl_macro_pre_authorize(macro_attrs: TokenStream, item_fn: ItemFn) -> TokenStream {
    Logic::generate(|| {
        let vis = &item_fn.vis;
        let sig = &item_fn.sig;
        let attrs = &item_fn.attrs;
        let block = &item_fn.block;

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

        let parsed = PreAuthorizeAttr::from_tokens(macro_attrs.clone().into())?;
        if parsed.ignore.unwrap_or(false) {
            return Ok(quote! { #(#attrs)* #vis #sig #block });
        }

        let warning_name = syn::Ident::new("__next_web_pre_authorize_warning", sig.ident.span());
        Ok(quote! {
            #(#attrs)*
            #vis #sig {
                #[deprecated(note = "#[pre_authorize] is parsed but not enforced yet; authorization checks are skipped")]
                fn #warning_name() {}
                #warning_name();
                #block
            }
        })
    })
}
