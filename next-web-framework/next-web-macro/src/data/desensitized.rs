use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Field, Ident};
use syn::spanned::Spanned;


pub(crate) fn impl_macro_desensitized(input: &syn::DeriveInput) -> TokenStream {
    let name = &input.ident;
    let fields = match &input.data {
        Data::Struct(data_struct) => &data_struct.fields,
        _ => panic!("error: #[derive(Desensitized)] is only supported for structs"),
    };

    // 为每个字段生成脱敏处理代码
    let desensitize_fields = fields.iter().map(|field| {
        let field_ident = field.ident.as_ref().expect("Fields should have identifiers");
        let field_type = &field.ty;
        
        // 查找字段的脱敏属性
        let desens_type = find_desensitization_type(field);
        
        match desens_type {
            Some(desens_type) => {
                // 根据不同的脱敏类型生成不同的处理逻辑
                match desens_type.as_str() {
                    "email" => quote! {
                        #field_ident: self.#field_ident.desensitize_email()
                    },
                    "phone" => quote! {
                        #field_ident: self.#field_ident.desensitize_phone()
                    },
                    "name" => quote! {
                        #field_ident: self.#field_ident.desensitize_name()
                    },
                    "partial" => {
                        // 获取部分脱敏的参数（如保留前几位，后几位）
                        // let (prefix, suffix) = get_partial_params(field);
                        let (prefix, suffix) = (0 , 1);
                        quote! {
                            #field_ident: self.#field_ident.desensitize_partial(#prefix, #suffix)
                        }
                    }
                    custom_type => quote! {
                        #field_ident: self.#field_ident.desensitize_custom(stringify!(#custom_type))
                    },
                }
            }
            None => {
                // 如果没有脱敏属性，直接使用原值
                quote! {
                    #field_ident: self.#field_ident.clone()
                }
            }
        }
    });

    // 生成结构体的脱敏实现
    let expanded = quote! {
        impl Desensitized for #name {
            fn desensitize(&self) -> Self {
                Self {
                    #(#desensitize_fields,)*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// 查找字段的脱敏类型
fn find_desensitization_type(field: &Field) -> Option<String> {
    for attr in &field.attrs {
        if attr.path().is_ident("de") {

            println!("attr: {:#?}", attr);
            // let meta = attr.parse_meta().expect("Failed to parse attribute meta");
            // if let syn::Meta::List(meta_list) = meta {
            //     if let Some(nested_meta) = meta_list.nested.first() {
            //         if let syn::NestedMeta::Meta(syn::Meta::Path(path)) = nested_meta {
            //             return Some(path.get_ident().unwrap().to_string());
            //         }
            //     }
            // }
        }
    }
    None
}

// fn get_partial_params(field: &Field) -> (usize, usize) {
//     for attr in &field.attrs {
//         if attr.path().is_ident("de") {
//             let meta = attr.parse_meta().expect("Failed to parse attribute meta");
//             if let syn::Meta::List(meta_list) = meta {
//                 if let Some(syn::NestedMeta::Meta(syn::Meta::NameValue(name_value))) = meta_list.nested.first() {
//                     if name_value.path.is_ident("partial") {
//                         if let syn::Lit::Str(lit_str) = &name_value.lit {
//                             let value = lit_str.value();
//                             let parts: Vec<&str> = value.split(',').collect();
//                             if parts.len() == 2 {
//                                 let prefix = parts[0].trim().parse().unwrap_or(3);
//                                 let suffix = parts[1].trim().parse().unwrap_or(4);
//                                 return (prefix, suffix);
//                             }
//                         }
//                     }
//                 }
//             }
//         }
//     }
//     (3, 4) // 默认值
// }