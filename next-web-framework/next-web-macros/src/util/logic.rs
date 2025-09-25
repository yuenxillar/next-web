use proc_macro2::TokenStream as TokenStream2;
use proc_macro::TokenStream;


pub struct Logic;

impl Logic {
    pub fn generate<F>(mut logic: F) -> TokenStream
    where
        F: FnMut() -> Result<TokenStream2, syn::Error>,
    {
        match logic() {
            Ok(stream) => stream.into(),
            Err(e) => e.to_compile_error().into(),
        }
    }
}