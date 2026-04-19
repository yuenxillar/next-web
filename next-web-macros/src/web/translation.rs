use proc_macro::TokenStream;
use quote::quote;
use syn::{Block, ItemFn, Stmt};

use crate::util::logic::Logic;

pub fn impl_macro_translation(_attr: TokenStream, mut item_fn: ItemFn) -> TokenStream {
    Logic::generate(|| {
        Logic::valid_method_handler(&item_fn)?;

        Logic::add_args(&mut item_fn,[
            quote! {
                ::next_web::i18n::AcceptHeaderLocaleResolver(__locale): ::next_web::i18n::AcceptHeaderLocaleResolver
            },
        ].into_iter());

        let stmts = item_fn.block.stmts.iter();
        let block: Block = syn::parse2(quote! {
          {
              ::next_web::i18n::RequestLocaleHolder::scope(__locale, async move {
                #(#stmts)*
            }).await
          }
        })
        .unwrap();
        item_fn.block = Box::new(block);

        let expanded = quote! {
            #item_fn
        };

        // println!("{}", expanded);

        Ok(expanded)
    })
}
