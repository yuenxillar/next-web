use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::ItemFn;

use crate::util::logic::Logic;

pub fn impl_macro_translation(attr: TokenStream, item_fn: ItemFn) -> TokenStream {
    Logic::generate(|| {
        Logic::valid_method_handler(&item_fn)?;
        // Logic::add_args(item_fn, args);
        let expanded = quote! {

            #item_fn
        };

        Ok(expanded)
    })
}
