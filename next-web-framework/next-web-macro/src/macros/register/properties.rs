// use proc_macro::TokenStream;
// use syn::{parse_macro_input, Error, Expr, Item, Meta, Token};
// use syn::parse::{Parse, ParseStream};
// use syn::parse::Parser;
// use syn::spanned::Spanned;

// pub struct Attribute {
//     prefix: String,
// }
// impl Parse for Attribute {
//     fn parse(input: ParseStream) -> Result<Self> {
//         // 解析 #[Properties(...)] 的内容
//         let meta = input.parse::<Meta>()?;
//         match &meta {
//             Meta::List(list) => {
//                 // 确保属性名是 "Properties"
//                 if list.path.is_ident("Properties") {
//                     println!("{:?}", list);
//                 }
//                 Err(Error::new(meta.span(), "Expected #[Properties(prefix = \"...\")]"))
//             }
//             _ => Err(Error::new(meta.span(), "Expected #[Properties(...)]")),
//         }
//     }
// }


// pub fn impl_macro_properties(attr: TokenStream, item: TokenStream) -> TokenStream {

//     let item = parse_macro_input!(item as Item);
//     let attr = parse_macro_input!(attr as Attribute);
//     let item_struct = match item {
//         Item::Struct(item_struct) => item_struct,
//         _ => panic!("impl_macro_properties only implemented for structs")
//     };

//     let struct_name = item_struct.ident.to_string();
//     //
//     // println!("Ping: {:?}", attr.to_string());
//     TokenStream::new()
// }