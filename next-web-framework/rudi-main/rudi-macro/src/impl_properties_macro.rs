use crate::PropertiesAttr;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Fields, ItemStruct, LitStr};

pub fn generate(attr: PropertiesAttr, mut item_struct: ItemStruct) -> syn::Result<TokenStream> {
    let required_derives = vec!["Debug", "Clone", "Deserialize", "Default"];
    let existing_derives = item_struct
        .clone()
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("derive"))
        .filter_map(|attr| {
            let meta = attr.meta.require_list().ok()?;
            Some(meta.tokens.to_string())
        })
        .collect::<Vec<_>>();

    println!("item_struct: {:#?}", item_struct.attrs);
    println!("existing_derives: {:#?}", existing_derives);

    println!("required_derives: {:#?}", required_derives);
    
    for required in required_derives {
        if !existing_derives.iter().any(|d| d.contains(required)) {
            return Err(syn::Error::new(
                item_struct.ident.span(),
                format!("Missing required derive: {}", required),
            ));
        }
    }

    let prefix = attr.prefix.to_token_stream().to_string().replace("\"", "");

    // 先获取字段和值的映射关系
    let fields = match &item_struct.fields {
        Fields::Named(fields_named) => &fields_named.named,
        _ => {
            return Err(syn::Error::new(
                item_struct.ident.span(),
                "Only named fields are supported",
            ))
        }
    };

    // 生成字段访问和赋值代码
    let field_assignments = fields
        .iter()
        .filter_map(|field| {
            let field_name = field.ident.as_ref()?;
            let field_type = &field.ty;

            // 检查字段类型是否是 Option<T>
            let is_option = if let syn::Type::Path(type_path) = field_type {
                if let Some(path_segment) = type_path.path.segments.last() {
                    path_segment.ident == "Option"
                } else {
                    false
                }
            } else {
                false
            };

            // 提取 Option<T> 中的 T 类型
            let inner_type = if is_option {
                if let syn::Type::Path(type_path) = field_type {
                    if let Some(path_segment) = type_path.path.segments.last() {
                        if let syn::PathArguments::AngleBracketed(args) = &path_segment.arguments {
                            if let Some(arg) = args.args.first() {
                                if let syn::GenericArgument::Type(inner) = arg {
                                    Some(inner)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            // 获取 #[value = "..."] 属性
            let value_attr = field
                .attrs
                .iter()
                .find(|attr| attr.path().is_ident("value"));

            // 如果找到了 value 属性，则使用它的值，否则使用字段名
            let key_name = if let Some(attr) = value_attr {
                if let Ok(meta) = attr.meta.require_name_value() {
                    if let syn::Expr::Lit(expr_lit) = &meta.value {
                        if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                            lit_str.value()
                        } else {
                            field_name.to_string()
                        }
                    } else {
                        field_name.to_string()
                    }
                } else {
                    field_name.to_string()
                }
            } else {
                field_name.to_string()
            };

            // 根据前缀是否存在决定键的格式
            let key = if prefix.is_empty() {
                key_name
            } else {
                format!("{}.{}", &prefix, key_name)
            };

            let key_str = LitStr::new(&key, field_name.span());

            // 根据字段类型生成不同的代码
            if is_option {
                let inner = inner_type.unwrap();
                Some(quote! {
                    // 对于 Option<T> 字段，直接将 Option<T> 赋值给它
                    #field_name: properties.one_value::<#inner>(#key_str),
                })
            } else {
                Some(quote! {
                    // 对于普通类型 T，需要检查 Option<T> 是否有值
                    #field_name: if let Some(value) = properties.one_value::<#field_type>(#key_str) {
                        value
                    }else {
                        Default::default()
                    },
                })
            }
        })
        .collect::<Vec<_>>();

    // 生成代码后，删除字段上的 value 属性
    if let Fields::Named(fields_named) = &mut item_struct.fields {
        for field in &mut fields_named.named {
            field.attrs.retain(|attr| !attr.path().is_ident("value"));
        }
    }

    let struct_name = &item_struct.ident;


    // 检查是否有 #[Singleton(name = "")] 属性
    let singleton_name = item_struct.attrs.iter().find_map(|attr| {
        if !attr.path().is_ident("Singleton") && !attr.path().is_ident("SingleOwner") {
            return None;
        }

        // 使用 parse_nested_meta 进行更安全的解析
        let mut name = None;
        let mut binds_exist = false;
        let _ = attr.parse_nested_meta(|meta| {

            if meta.path.is_ident("name") {
                let value = meta.value()?;
                let string_value = value.parse::<syn::LitStr>()?;
                name = Some(string_value.value());
                return Ok(());
            }

            if meta.path.is_ident("binds") {
                binds_exist = true;
                let value = meta.value()?;
                let bind_array = value.parse::<syn::ExprArray>()?;
                let binds = bind_array.to_token_stream().to_string();
                if !binds.contains("into_properties") {
                    panic!("Singleton or SingleOwner macro must contain ::into_properties");
                }
            }
            Ok(())
        });
        if !binds_exist {
            panic!("Singleton or SingleOwner macro must support binds `#[Singleton(binds = [Self::into_properties])]`")
        }
        name
    });

    // 如果没有找到 Singleton 属性或者 name 参数，生成默认名称
    let default_name = {
        let struct_name_str = struct_name.to_string();
        let mut chars = struct_name_str.chars();
        // 将首字母小写
        if let Some(first_char) = chars.next() {
            let first_char_lower = first_char.to_lowercase().to_string();
            first_char_lower + chars.as_str()
        } else {
            struct_name_str
        }
    };

    let singleton_name_value = if let Some(name) = singleton_name {
        name
    } else {
        default_name
    };

    let singleton_name = LitStr::new(&singleton_name_value, struct_name.span());

    let expanded = quote! {
        #item_struct

        #[next_web_core::async_trait]
        impl next_web_core::AutoRegister for #struct_name {
            async fn register(
                &self,
                ctx: &mut next_web_core::context::application_context::ApplicationContext,
                properties: &next_web_core::context::properties::ApplicationProperties,
            ) -> Result<(), Box<dyn std::error::Error>> {
                
                let instance = Self {
                    #(#field_assignments)*

                };
                ctx.insert_singleton_with_name(instance, self.singleton_name());
                Ok(())
            }

            fn singleton_name(&self) -> &'static str {
                #singleton_name
            }
        }

        impl next_web_core::context::properties::Properties for #struct_name {}

        impl #struct_name {
            fn into_properties(self) -> Box<dyn next_web_core::context::properties::Properties> {
                std::boxed::Box::new(self)
            }
        }
    };

    println!("cfff:{}", expanded.to_string());
    Ok(expanded.into())
}
