use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Error, FnArg, ItemFn};
use syn::spanned::Spanned;

pub struct Logic;

impl Logic {
    pub fn generate<F>(mut logic: F) -> TokenStream
    where
        F: FnMut() -> Result<TokenStream2, Error>,
    {
        match logic() {
            Ok(stream) => stream.into(),
            Err(e) => e.to_compile_error().into(),
        }
    }

    pub fn valid_method_handler(item_fn: &ItemFn) -> Result<(), Error> {
        if item_fn.sig.asyncness.is_none() {
            return Err(Error::new(
                item_fn.sig.span(),
                "Function must be declared as async",
            ));
        }

        if matches!(item_fn.sig.output, syn::ReturnType::Default) {
            return Err(Error::new_spanned(
                item_fn,
                "Function has no return type. Cannot be used as handler",
            ));
        }

        let inputs = &item_fn.sig.inputs;
        if inputs.iter().any(|val| match val {
            FnArg::Receiver(_) => true,
            _ => false,
        }) {
            return Err(Error::new(
                inputs.span(),
                "The function must not have a receiver, self",
            ));
        }

        Ok(())
    }


    pub fn add_args<A>(item_fn: &mut ItemFn, args: A)
    where
        A: Iterator<Item = proc_macro2::TokenStream>,
    {
        for (index, arg) in args.enumerate() {
            // println!("adding arg: {}", arg.to_string());
            item_fn.sig.inputs.insert(index, syn::parse2(arg).unwrap());
        }
    }


    pub fn add_block(item_fn: &mut ItemFn, block: TokenStream2) {
        let default_block = item_fn.block.stmts.clone();
        *item_fn.block = syn::parse2(
            quote! {
                {
                    #block

                    #(#default_block)*
                }
            }
        ).unwrap();
    }
}
