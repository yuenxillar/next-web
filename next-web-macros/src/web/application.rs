use proc_macro::TokenStream;
use quote::quote;
use syn::ItemFn;

pub fn impl_macro_application(attrs: TokenStream, item_fn: ItemFn) -> TokenStream {
    #[cfg(feature = "embed-resources")]
    let expanded = quote! {


        #item_fn
    };

    #[cfg(not(feature = "embed-resources"))]
    let expanded = quote! {

        #[derive(::next_web::macros::embed::Embed)]
        #[folder = "resources/"]
        #[include = "*.html"]
        #[include = "*.json"]
        #[include = "*.xml"]
        #[include = "*.yaml"]
        #[include = "*.yml"]
        #[include = "*.properties"]
        #[include = "*.toml"]
        #[include = "*.txt"]
        pub struct _ApplicationResources;

        #item_fn
    };

    TokenStream::from(expanded)
}
