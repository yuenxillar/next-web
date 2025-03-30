use from_attr::{AttrsValue, FromAttr};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, GenericParam, ReturnType, Visibility};

use crate::macros::dependency_injection::{bean::ClosureOrPath, color::Color, common_utils::CommonUtils, commons::{self, ArgumentResolveStmts}};

use super::{ autowired_attr::AutoWiredAttr, bean::FunctionAttr, scope::Scope};

pub(super) fn generate(
    attr: FunctionAttr,
    mut item_fn: syn::ItemFn,
    scope: Scope,
) -> syn::Result<TokenStream> {
    // 判断函数可见性
    match item_fn.vis {
        Visibility::Public(_) => (),
        _ => {
            return Err(syn::Error::new(
                item_fn.span(),
                "The visibility of the function must be public",
            ));
        }
    };

    // 判断函数是否有返回值，若没有则显示错误信息
    if let syn::ReturnType::Default = item_fn.sig.output {
        return Err(syn::Error::new(
            item_fn.span(),
            "The function must have a return type",
        ));
    };

    let AutoWiredAttr { path } = match AutoWiredAttr::remove_attributes(&mut item_fn.attrs) {
        Ok(Some(AttrsValue { value: attr, .. })) => attr,
        Ok(None) => AutoWiredAttr::default(),
        Err(AttrsValue { value: e, .. }) => return Err(e),
    };

    let FunctionAttr {
        name,
        condition,
        order,
        ..
    } = attr;


    // 获取函数的异步类型
    let color = match item_fn.sig.asyncness {
        Some(_) => Color::Async,
        None => Color::Sync,
    };

    // 判断作用域是否为单例，若是原型需要和 name 属性互斥
    if scope == Scope::Prototype && name.is_some() {
        return Err(syn::Error::new(
            name.span(),
            "The name attribute cannot be used with the prototype scope",
        ));
    }

    // 判断name属性是否存在，若不存在则使用函数名作为bean名称
    let bean_name = name.unwrap_or_else(|| {
        let name = item_fn.sig.ident.to_string();
        let bean_name = CommonUtils::generate_bean_name(&name);
        syn::parse_str(&bean_name).unwrap()
    });

    // 判断条件是否成立，否则不做注册处理
    let condition = condition
        .map(|ClosureOrPath(expr)| quote!(Some(#expr)))
        .unwrap_or_else(|| quote!(None));


    // 生成参数解析方法
    let ArgumentResolveStmts {
        ref_mut_cx_stmts,
        ref_cx_stmts,
        args,
    } = commons::generate_argument_resolve_methods(&mut item_fn.sig.inputs, color)?;


    // 生成创建bean的方法
    let create_provider = commons::generate_create_provider(scope, color);

    let (impl_generics, ty_generics, where_clause) = item_fn.sig.generics.split_for_impl();

    // 获取函数可见性
    let vis = &item_fn.vis;

    // 获取文档注释
    let docs = item_fn
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"));

    let ident = &item_fn.sig.ident;

    let return_type_ident = match &item_fn.sig.output {
        ReturnType::Default => quote! {
            ()
        },
        ReturnType::Type(_, ty) => quote! {
            #ty
        },
    };

    let struct_definition = if item_fn.sig.generics.params.is_empty() {
        quote! {
            #vis struct #ident;
        }
    } else {
        let members = item_fn
            .sig
            .generics
            .params
            .iter()
            .filter_map(|param| match param {
                GenericParam::Type(ty) => Some(ty),
                _ => None,
            })
            .enumerate()
            .map(|(idx, ty)| {
                let ty_ident = &ty.ident;
                let ident = quote::format_ident!("_mark{}", idx);
                quote! { #ident: ::core::marker::PhantomData<#ty_ident> }
            });

        quote! {
            #[derive(Default)]
            #vis struct #ident #ty_generics { #(#members),*}
        }
    };

    // 生成泛型参数
    let turbofish = ty_generics.as_turbofish();

    // 生成构造函数
    let constructor = match color {
        Color::Async => {
            quote! {
                #[allow(unused_variables)]
                |cx| ::std::boxed::Box::pin(async {
                    #(#ref_mut_cx_stmts)*
                    #(#ref_cx_stmts)*
                    #ident #turbofish (#(#args,)*).await
                })
            }
        }
        Color::Sync => {
            quote! {
                #[allow(unused_variables)]
                |cx| {
                    #(#ref_mut_cx_stmts)*
                    #(#ref_cx_stmts)*
                    #ident #turbofish (#(#args,)*)
                }
            }
        }
    };

    let auto_register = quote! {
        #path::register_provider!(<#ident as #path::DefaultProvider>::provider());
    };

    let expand = quote! {
        #(#docs)*
        #[allow(non_camel_case_types)]
        #struct_definition

        impl #impl_generics #path::DefaultProvider for #ident #ty_generics #where_clause {
            type Type = #return_type_ident;

            fn provider() -> #path::Provider<Self::Type> {
                #[allow(non_snake_case, clippy::too_many_arguments)]
                #item_fn

                <#path::Provider<_> as ::core::convert::From<_>>::from(
                    #path::#create_provider(#constructor)
                        .name(#bean_name)
                        .order(#order)
                        .condition(#condition)
                )
            }
        }
        #auto_register
    };

    Ok(expand)
}
