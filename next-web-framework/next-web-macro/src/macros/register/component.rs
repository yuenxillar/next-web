use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens, format_ident};
use syn::{parse::Parse, parse_macro_input, Attribute, Expr, Ident, ItemImpl, ItemStruct, LitStr, Token};
use syn::parse::{ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::Comma;

// 宏的参数解析结构
pub struct ComponentArgs {
    name: Option<String>,
    scope: Option<String>,
    lazy: bool,
}

impl Parse for ComponentArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        // 处理空参数情况
        if input.is_empty() {
            return Ok(ComponentArgs { name: None, scope: None, lazy: false });
        }
        
        // 处理单个字符串情况
        if input.peek(LitStr) {
            let value: LitStr = input.parse()?;
            return Ok(ComponentArgs { 
                name: Some(value.value()), 
                scope: None, 
                lazy: false 
            });
        }
        
        // 处理原始的键值对格式...（现有代码）
        let content;
        syn::parenthesized!(content in input);
        
        let mut name = None;
        let mut scope = None;
        let mut lazy = false;
        
        let args = Punctuated::<ComponentArg, Token![,]>::parse_terminated(&content)?;
        for arg in args {
            match arg {
                ComponentArg::Name(value) => name = Some(value),
                ComponentArg::Scope(value) => scope = Some(value),
                ComponentArg::Lazy(value) => lazy = value,
            }
        }

        Ok(ComponentArgs { name, scope, lazy })
    }
}

// 各种参数类型
enum ComponentArg {
    Name(String),
    Scope(String),
    Lazy(bool),
}

impl Parse for ComponentArg {
    fn parse(input: ParseStream) -> Result<Self> {
        let key: Ident = input.parse()?;
        let key_str = key.to_string();

        match key_str.as_str() {
            "name" => {
                input.parse::<Token![=]>()?;
                let value: LitStr = input.parse()?;
                Ok(ComponentArg::Name(value.value()))
            },
            "scope" => {
                input.parse::<Token![=]>()?;
                let value: LitStr = input.parse()?;
                Ok(ComponentArg::Scope(value.value()))
            },
            "lazy" => {
                if input.peek(Token![=]) {
                    input.parse::<Token![=]>()?;
                    let value: syn::LitBool = input.parse()?;
                    Ok(ComponentArg::Lazy(value.value))
                } else {
                    Ok(ComponentArg::Lazy(true))
                }
            },
            _ => Err(syn::Error::new(key.span(), format!("Unknown parameter: {}", key_str))),
        }
    }
}

// 处理作用于结构体的 Component 宏
pub fn component_struct(args: ComponentArgs, input: ItemStruct) -> TokenStream {
    let struct_name = &input.ident;
    let struct_name_str = struct_name.to_string();
    
    // 组件名称，默认为结构体名转小写
    let component_name = args.name.unwrap_or_else(|| struct_name_str.to_lowercase());
    
    // 定义作用域，默认为单例
    let scope = args.scope.unwrap_or_else(|| "singleton".to_string());
    
    let register_fn_name = format_ident!("register_{}", struct_name);
    
    // 生成注册代码
    let registration = match scope.as_str() {
        "singleton" => quote! {
            #[ctor::ctor]
            fn #register_fn_name() {
                let context = ApplicationContext::get_global_context();
                let component = #struct_name::new();
                let _ = context.register_with_name(#component_name, component);
            }
        },
        "prototype" => quote! {
            // 原型模式不会预先注册实例，而是提供工厂方法
            #[ctor::ctor]
            fn #register_fn_name() {
                let context = ApplicationContext::get_global_context();
                let factory = || #struct_name::new();
                let _ = context.register_factory(#component_name, Box::new(factory));
            }
        },
        _ => panic!("Unsupported scope: {}", scope),
    };
    
    // 生成最终代码
    quote! {
        #input
        
        #registration
        
        impl #struct_name {
            // 为组件添加一个静态获取方法
            pub fn get_component() -> #struct_name {
                let context = ApplicationContext::get_global_context();
                context.get_single_with_name::<#struct_name>(#component_name)
                    .expect(&format!("无法获取组件: {}", #component_name))
            }
        }
    }
}

// 处理作用于实现块的 Component 宏
pub fn component_impl(args: ComponentArgs, input: ItemImpl) -> TokenStream {
    let impl_type = &input.self_ty;
    let type_name = impl_type.to_token_stream().to_string();
    
    // 组件名称，默认为类型名转小写
    let component_name = args.name.unwrap_or_else(|| type_name.to_lowercase());
    
    let component_name_str = component_name.clone();
    let register_fn_name = format_ident!("register_{}", component_name_str);
    
    // 依赖注入相关方法
    let inject_methods = quote! {
        // 获取当前组件
        pub fn get_component() -> Self {
            let context = ApplicationContext::get_global_context();
            context.get_single_with_name::<Self>(#component_name)
                .expect(&format!("无法获取组件: {}", #component_name))
        }
        
        // 注入依赖
        pub fn inject<T: 'static + Send + Sync + Clone>(&self, name: &str) -> Option<T> {
            let context = ApplicationContext::get_global_context();
            context.get_single_option_with_name::<T>(name)
        }
    };
    
    // 组合原有实现和新增方法
    quote! {
        #input
        
        impl #impl_type {
            #inject_methods
        }
        
        #[ctor::ctor]
        fn #register_fn_name() {
            let context = ApplicationContext::get_global_context();
            let component = #impl_type::new();
            let _ = context.register_with_name(#component_name, component);
        }
    }
}

// 宏入口函数
pub fn impl_macro_component(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(attr as ComponentArgs);
    
    let output = match syn::parse::<syn::Item>(item.clone()) {
        Ok(syn::Item::Struct(input)) => component_struct(args, input),
        Ok(syn::Item::Impl(input)) => component_impl(args, input),
        _ => panic!("Component 宏只能用于结构体或实现块"),
    };
    
    proc_macro::TokenStream::from(output)
} 