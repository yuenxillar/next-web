use proc_macro::TokenStream;


#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn SingleOwner(attr: TokenStream, item: TokenStream) -> TokenStream {
    // generate(attr, item)
    TokenStream::new()
}