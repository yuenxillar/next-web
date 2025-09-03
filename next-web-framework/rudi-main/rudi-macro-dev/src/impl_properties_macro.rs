use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Fields, ItemStruct, LitStr};

use crate::{
    util::{extract_option_inner_type, is_string},
    PropertiesAttr,
};

pub fn generate(attr: PropertiesAttr, mut item_struct: ItemStruct) -> syn::Result<TokenStream> {
    let required_derives = vec!["Debug", "Clone", "Deserialize"];
    let existing_derives = &item_struct
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("derive"))
        .filter_map(|attr| {
            let meta = attr.meta.require_list().ok()?;
            Some(meta.tokens.to_string())
        })
        .collect::<Vec<_>>();
    for required in required_derives {
        if !existing_derives.iter().any(|d| d.contains(required)) {
            return Err(syn::Error::new(
                item_struct.ident.span(),
                format!("Missing required derive: {}", required),
            ));
        }
    }

    let prefix_expr = attr.prefix;

    let prefix = prefix_expr.to_token_stream().to_string().replace("\"", "");
    let dynamic = attr.dynamic;

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
    // Generate field access and assignment code
    let common_fields = fields
        .iter()
        .enumerate()
        .filter(|(_, field)| field.ident.is_some())
        .filter_map(|(index, field)| {
            // （如果是 dynamic） 跳过第一个字段
            if dynamic && index == 0 {
                return None;
            }

            let field_name = field.ident.as_ref()?;
            let field_type = &field.ty;

            // 检查是否为 Option<T>
            let (is_option, inner_type) = match extract_option_inner_type(field_type) {
                Some(ty) => (true, ty),
                None => (false, field_type.clone()),
            };

            // 获取 key 属性或使用字段名
            let key_name = field
                .attrs
                .iter()
                .find(|attr| attr.path().is_ident("key"))
                .and_then(|attr| {
                    attr.meta.require_name_value().ok().and_then(|meta| {
                        if let syn::Expr::Lit(expr_lit) = &meta.value {
                            if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                                return Some(lit_str.value());
                            }
                        }
                        None
                    })
                })
                .unwrap_or_else(|| field_name.to_string());

            // 构建最终 key
            let key = if prefix.is_empty() {
                key_name
            } else {
                format!("{}.{}", prefix, key_name)
            };
            let key_str = LitStr::new(&key, field_name.span());

            // 判断是否为 String 类型（考虑 Option<String>）
            let is_string_type = is_string(&inner_type);

            // 生成核心表达式：从 properties 中提取值
            let extract_value_expr = if is_string_type {
                // String 类型需要支持数字转字符串
                quote! {
                    || -> Option<String> {
                        // 优先尝试 one_value
                        if let Some(s) = properties.one_value::<String>(#key_str) {
                            return Some(s);
                        }

                        match properties.one_value::<String>(#key_str) {
                            Some(s) => Some(s),
                            None => {
                                match properties.one_value::<i64>(#key_str) {
                                    Some(s) => Some(s.to_string()),
                                    None => match properties.one_value::<f64>(#key_str) {
                                            Some(s) => Some(s.to_string()),
                                            None => None,
                                    }
                                }
                            }
                        }
                    }()
                }
            } else {
                // 非字符串类型：直接尝试 one_value
                quote! {
                    properties.one_value::<#inner_type>(#key_str)
                }
            };

            // 根据 Option<T> 和 T 生成最终字段赋值
            let field_init = if is_option {
                quote! { #field_name: #extract_value_expr, }
            } else {
                quote! {
                    #field_name: #extract_value_expr.unwrap_or_else(|| {
                        noting = true;
                        Default::default()
                    }),
                }
            };

            Some(field_init)
        })
        .collect::<Vec<_>>();

    // dynamic_field
    let dynamic_field = if dynamic {
        quote! {
            base: if let Some(values) = properties.dynamic_value(#prefix_expr) { values } else { Default::default() },
        }
    } else {
        quote! {}
    };

    // 生成代码后，删除字段上的 key 属性
    if let Fields::Named(fields_named) = &mut item_struct.fields {
        for field in &mut fields_named.named {
            field.attrs.retain(|attr| !attr.path().is_ident("key"));
        }
    }

    let struct_ident = &item_struct.ident;

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
                    return Err(syn::Error::new(Span::call_site(), "Singleton or SingleOwner macro must contain ::into_properties"));
                }
            }
            Ok(())
        });
        if !binds_exist {
            panic!("Singleton or SingleOwner macro must support binds `#[Singleton(binds = [Self::into_properties])]`");
        }
        name
    });

    // 如果没有找到 Singleton 属性或者 name 参数，生成默认名称

    let singleton_name = if let Some(name) = singleton_name {
        name
    } else {
        // default_name
        super::util::singleton_name(&struct_ident.to_string())
    };

    let singleton_name = LitStr::new(&singleton_name, struct_ident.span());

    let expanded = quote! {
        #item_struct

        #[next_web_core::async_trait]
        impl ::next_web_core::AutoRegister for #struct_ident {
            async fn register(
                &self,
                ctx: &mut ::next_web_core::context::application_context::ApplicationContext,
                properties: & ::next_web_core::context::properties::ApplicationProperties,
            ) -> ::std::result::Result<(), ::std::boxed::Box<dyn ::std::error::Error>> {
                let mut noting = false;

                let instance = Self {
                    #dynamic_field

                    #(#common_fields)*
                };

                if noting {
                    panic!("\nIncorrect assembly of properties! Struct: {} \n", stringify!(#struct_ident));
                }

                ctx.insert_singleton_with_name(instance, #singleton_name);
                Ok(())
            }

            fn registered_name(&self) -> &'static str {
                #singleton_name
            }
        }

        impl ::next_web_core::context::properties::Properties for #struct_ident {}

        impl #struct_ident {
            fn into_properties(self) -> ::std::boxed::Box<dyn ::next_web_core::context::properties::Properties> {
                ::std::boxed::Box::new(self)
            }
        }
    };

    // println!("expanded: {}", expanded);

    Ok(expanded.into())
}
