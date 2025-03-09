use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, Fields, Ident};

pub(crate) fn impl_macro_builder(input: &syn::DeriveInput) -> TokenStream {
    let struct_name = &input.ident;

    match &input.data {
        Data::Struct(_) => (),
        _ => panic!("error: #[derive(Builder)] is only supported for structs"),
    };

    // 生成 Builder 名称
    let builder_name = Ident::new(&format!("{}Builder", struct_name), struct_name.span());

    // 提取结构体字段
    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(named_fields) => &named_fields.named,
            _ => panic!("Builder only supports structs with named fields"),
        },
        _ => panic!("Builder only supports structs"),
    };

    // 生成字段和对应的 Option<T> 字段
    let builder_fields = fields.iter().map(|field| {
        let field_name = &field.ident;
        let field_type = &field.ty;

        // 检查字段类型是否为 Option<T>
        if is_option_type(field_type) {
            quote! {
                #field_name: #field_type // 如果是 Option<T>，直接使用原类型
            }
        } else {
            quote! {
                #field_name: std::option::Option<#field_type> // 否则包装为 Option<T>
            }
        }
    });

    // 生成字段的 setter 方法
    let setter_methods = fields.iter().map(|field| {
        let field_name = &field.ident;
        let field_type = &field.ty;

        if is_option_type(field_type) {
            // 如果字段是 Option<T>，直接赋值
            quote! {
                pub fn #field_name(&mut self, value: #field_type) -> &mut Self {
                    self.#field_name = value; // 不需要再包装为 Some
                    self
                }
            }
        } else {
            // 如果字段不是 Option<T>，包装为 Some
            quote! {
                pub fn #field_name(&mut self, value: #field_type) -> &mut Self {
                    self.#field_name = Some(value);
                    self
                }
            }
        }
    }).collect::<Vec<proc_macro2::TokenStream>>();

    // let _ = setter_methods.iter().for_each(|f| println!("var1: {}", f.to_string()));
    // 生成 build 方法
    let build_method = {
        let field_initializers = fields.iter().map(|field| {
            let field_name = &field.ident;
            let field_type = &field.ty;
            if is_option_type(field_type) {
                quote! {
                    #field_name: Some(self.#field_name.take().ok_or(format!(concat!(stringify!(#field_name), " is not set")))?)
                }
            }else {
                quote! {
                    #field_name: self.#field_name.take().ok_or(format!(concat!(stringify!(#field_name), " is not set")))?
                }
            }
            
        });
        quote! {
            pub fn build(&mut self) -> std::result::Result<#struct_name, String> {
                Ok(#struct_name {
                    #( #field_initializers ),*
                })
            }
        }
    };
  

    // 生成 Builder 初始化代码
    // 在 builer_fields 生成部分调整：
    let builer_fields = fields.iter().map(|field| {
    let field_name = &field.ident;
    
    // 检查是否有 #[builder(default = "...")] 属性
    let mut is_default = 0;
    let mut expr_value = None;
    for attr in field.attrs.iter() {
        if attr.path().is_ident("builder") {
            let _ = attr.parse_nested_meta(|meta| { 
                if meta.path.is_ident("default") {  
                    if let Err(_) = meta.value() {
                        is_default = 1;
                        return Ok(());
                    }
                    let s: syn::Expr = meta.value()?.parse()?; 
                    expr_value = Some(s);
                }
                Ok(())
            }); 
        }
    }

    if let Some(expr) = expr_value {
        // 使用用户指定的函数初始化
        quote! { #field_name: Some(#expr) }
    } else 
    if is_default == 1 {
        // 使用 Default::default()
        quote! { #field_name: Some(Default::default()) }
    } else {
        // 初始化为 None
        quote! { #field_name: None }
    }
    });

    // 生成完整的 Builder 实现
    let expanded = quote! {
        pub struct #builder_name {
            #( #builder_fields ),*
        }

        impl #builder_name {
            #( #setter_methods )*

            #build_method
        }

        impl #struct_name {
            pub fn builder() -> #builder_name {
                #builder_name {
                    #( #builer_fields ),*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

// 判断是否为 Option 类型（支持嵌套 Option）
fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                return true;
            }
            // 递归检查嵌套 Option
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                    return is_option_type(inner_type);
                }
            }
        }
    }
    false
}
