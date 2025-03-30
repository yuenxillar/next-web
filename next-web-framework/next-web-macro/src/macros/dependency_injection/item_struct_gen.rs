use proc_macro2::TokenStream;

use super::struct_or_function_attr::StructOrFunctionAttr;

pub(crate) fn generate(
    attr: StructOrFunctionAttr,
    item_struct: syn::ItemStruct,
    scope: super::scope::Scope,
) -> syn::Result<TokenStream> {
    println!("item_struct: {:?}", item_struct);

    println!("attr: {:?}", attr.name);
    println!("attr.scope: {:?}", attr.scope);
    Ok(TokenStream::new())
}
