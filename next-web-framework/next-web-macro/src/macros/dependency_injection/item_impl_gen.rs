use proc_macro2::TokenStream;

use super::struct_or_function_attr::StructOrFunctionAttr;

pub(crate) fn generate(
    attr: StructOrFunctionAttr,
    item_impl: syn::ItemImpl,
    scope: super::scope::Scope,
) -> syn::Result<TokenStream> {
    Ok(TokenStream::new())
}
