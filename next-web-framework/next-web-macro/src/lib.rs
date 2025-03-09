extern crate proc_macro;

use macros::data::builder::impl_macro_builder;
use macros::data::field_name::impl_macro_field_name;
use macros::data::get_set::impl_macro_get_set;
use macros::database::db_mapper::impl_macro_db_mapper;
use proc_macro::TokenStream;
use syn::parse_macro_input;
use syn::DeriveInput;

mod macros;
mod utils;

#[proc_macro_derive(GetSet, attributes(skip))]
pub fn get_set(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let stream = impl_macro_get_set(&input);
    stream.into()
}


#[proc_macro_derive(FieldName)]
pub fn field_name(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let stream = impl_macro_field_name(&input);
    stream.into()
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let stream = impl_macro_builder(&input);
    stream.into()
}



// database
#[proc_macro_derive(DbMapper, attributes(d))]
pub fn all_constructor(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let stream = impl_macro_db_mapper(&input);
    stream.into()
}



// //=============================================
// #[proc_macro_derive(Properties)]
// pub fn derive_properties(input: TokenStream) -> TokenStream {
//     // 解析输入的结构体
//     let ast = parse_macro_input!(input as DeriveInput);
//     let struct_name = &ast.ident;

//     // 获取结构体的字段
//     let fields = match &ast.data {
//         Data::Struct(data) => match &data.fields {
//             Fields::Named(fields) => &fields.named,
//             _ => panic!("Properties macro only supports structs with named fields."),
//         },
//         _ => panic!("Properties macro only supports structs."),
//     };

//     // 为每个字段生成默认值
//     let default_values = fields.iter().map(|field| {
//         let _field_name = &field.ident;
//         let field_type = &field.ty;

//         // 根据字段类型生成默认值
//         match field_type {
//             Type::Path(type_path) => {
//                 let type_name = type_path.path.segments.last().unwrap().ident.to_string();
//                 match type_name.as_str() {
//                     "String" => quote! { String::new() },
//                     "i32" | "u32" | "i64" | "u64" | "f32" | "f64" => quote! { 0 },
//                     "bool" => quote! { false },
//                     _ => quote! { <#field_type as Default>::default() },
//                 }
//             }
//             _ => quote! { <#field_type as Default>::default() },
//         }
//     });

//     // 生成 `Default` 实现
//     let expanded = quote! {
//         // 默认实现
//         impl Default for #struct_name {
//             fn default() -> Self {
//                 Self {
//                     #(
//                         #fields: #default_values,
//                     )*
//                 }
//             }
//         }

//         // 收集所有注册的结构体
//         inventory::collect!(#struct_name);
//     };

//     TokenStream::from(expanded)
// }
