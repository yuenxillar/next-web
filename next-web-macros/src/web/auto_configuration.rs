use std::collections::HashSet;

use from_attr::FromAttr;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{
    spanned::Spanned, Error, FnArg, Ident, ImplItem, ImplItemFn, ItemImpl, Lit, LitBool, LitFloat,
    LitInt, LitStr, Pat, PatType, ReturnType, Signature, Type,
};

use crate::{util::logic::Logic, web::attrs::auto_configuration_attr::*};

pub fn impl_macro_auto_configuration(_attrs: TokenStream, mut item_impl: ItemImpl) -> TokenStream {
    Logic::generate(|| {
        safety_check(&item_impl)?;

        let type_path = match item_impl.self_ty.as_ref() {
            Type::Path(type_path) => type_path.path.segments.last(),
            _ => return Err(Error::new(item_impl.self_ty.span(), "")),
        };

        let path = match type_path {
            Some(path) => path,
            None => return Err(Error::new(type_path.span(), "")),
        };

        let name = Ident::new(
            &format!("__{}Gen", path.ident.to_string()),
            Span::call_site(),
        );

        let mut attributes = attributes(&mut item_impl.items);

        let cfn = attributes
            .iter_mut()
            .map(|(_index, _attr, fn_attr)| fn_attr.provider_attr.as_mut())
            .map(|attr| {
                attr.map(|a| a.conditional.take().unwrap_or_default())
                    .unwrap_or_default()
            })
            .map(|attr| {
                attr.iter()
                    .filter_map(|expr| {
                        let s = expr.path.to_token_stream().to_string();
                        if !(s.contains("::")) {
                            Some(s)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect::<HashSet<String>>()
            .into_iter()
            .map(|s| Ident::new(&s, Span::call_site()))
            .collect::<Vec<_>>();

        let impl_blocks = gen_code(&item_impl.items, attributes)?;

        let conditional_function = item_impl
            .items
            .iter()
            .filter_map(|item| match item {
                ImplItem::Fn(impl_item_fn) => Some(impl_item_fn),
                _ => None,
            })
            .filter_map(|s| {
                if cfn.iter().any(|s1| &s.sig.ident == s1) {
                    return Some(s);
                }
                None
            })
            .collect::<Vec<_>>();

        println!("1");

        let expanded = quote! {

             struct #name;

             impl ::next_web_core::autoconfigure::default_auto_configure::DefaultAutoConfigure for #name {
                 fn auto_configure<'life_a>(
                     &'life_a self,
                     ctx: &'life_a mut ApplicationContext,
                 ) -> core::pin::Pin<std::boxed::Box<dyn Future<Output = ()> + Send + 'life_a>>
                 {
                      ::std::boxed::Box::pin(async {

                          #(#conditional_function)*

                          #(#impl_blocks);*

                      })
                 }
             }

             ::next_web_core::submit_default_auto_configure!(#name);
        };

        println!("expanded: {}", expanded.to_string());

        Ok(expanded)
    })
}

fn safety_check(item_impl: &ItemImpl) -> Result<(), Error> {
    // 1. Check if the trait has been implemented
    if let Some(trait_name) = match item_impl.trait_.as_ref() {
        Some((_, path, _)) => path.segments.last().map(|s| s.ident.to_string()),
        None => None,
    } {
        return Err(Error::new(
            Span::call_site(),
            format!(
                "`auto_configuration` macros are not supported for trait '{}' implementation",
                trait_name
            ),
        ));
    }

    // 2. Traverse all methods and check if there is a self parameter
    for item in item_impl.items.iter() {
        if let ImplItem::Fn(item_fn) = item {
            for fn_arg in item_fn.sig.inputs.iter() {
                match fn_arg {
                    FnArg::Receiver(receiver) => {
                        return Err(Error::new(
                           receiver.span(),
                               "The method of automatic configuration class cannot have a self parameter",
                       ));
                    }
                    FnArg::Typed(typed) => match typed.ty.as_ref() {
                        Type::Reference(refer) => {
                            if refer.mutability.is_some() {
                                return Err(Error::new(
                                   refer.span(),
                                   format!( "There are mutable parameters in the current function `{}`, which are not allowed and can only be kept as references or owners.", item_fn.sig.ident.to_string())
                               ));
                            }
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    Ok(())
}

fn get_item_fn_atrrs<'a>(impl_item_fn: &'a mut ImplItemFn) -> ItemFnAtrr {
    let provider_attr = match ProviderAttr::remove_attributes(&mut impl_item_fn.attrs) {
        Ok(attrs) => attrs.map(|attr| attr.value),
        Err(_err) => None,
    };

    let conditional_on_property_attr =
        match ConditionalOnPropertyAttr::remove_attributes(&mut impl_item_fn.attrs) {
            Ok(attrs) => attrs.map(|attr| attr.value),
            Err(_err) => None,
        };

    ItemFnAtrr {
        ident: impl_item_fn.sig.ident.clone(),
        _async: impl_item_fn.sig.asyncness.is_some(),
        sig: impl_item_fn.sig.clone(),
        provider_attr,
        conditional_on_property_attr,
    }
}

fn get_autowired_attr(pat_type: &mut PatType) -> Option<AutowiredAttr> {
    if let Some(attrs) = match AutowiredAttr::remove_attributes(&mut pat_type.attrs) {
        Ok(attr) => attr,
        Err(_err) => return None,
    } {
        return attrs.value.into();
    }

    None
}

fn attributes(items: &mut [ImplItem]) -> Vec<(usize, Vec<Option<AutowiredAttr>>, ItemFnAtrr)> {
    items
        .iter_mut()
        .filter_map(|item| match item {
            ImplItem::Fn(impl_item_fn) => Some(impl_item_fn),
            _ => None,
        })
        .filter(|item| !matches!(item.sig.output, ReturnType::Default))
        .filter(|item_fn| supported_attributes(item_fn))
        .enumerate()
        .map(|(index, item_fn)| {
            (
                index,
                item_fn
                    .sig
                    .inputs
                    .iter_mut()
                    .filter_map(|input| match input {
                        FnArg::Typed(typed) => Some(typed),
                        FnArg::Receiver(_) => None,
                    })
                    .map(|typ| get_autowired_attr(typ))
                    .collect::<Vec<_>>(),
                get_item_fn_atrrs(item_fn),
            )
        })
        .collect::<Vec<_>>()
}

fn gen_code(
    items: &[ImplItem],
    attributes: Vec<(usize, Vec<Option<AutowiredAttr>>, ItemFnAtrr)>,
) -> Result<Vec<TokenStream2>, Error> {
    let impl_blocks = items
        .iter()
        .filter_map(|item| match item {
            ImplItem::Fn(impl_item_fn) => Some(impl_item_fn),
            _ => None,
        })
        .filter(|item|!matches!(item.sig.output, ReturnType::Default))
        .zip(attributes)
        .enumerate()
        .filter(|(index, (_item_fn ,(i, _, _) ))| index == i)
        .map(|(_, (item_fn, (_, autowired_attrs, item_fn_attr)) )| {

            let ItemFnAtrr {
                ident,
                _async,
                sig,
                provider_attr,
                conditional_on_property_attr,
            } = item_fn_attr;


            let arg_names = (0..sig.inputs.len())
                .map(|index| Ident::new(&format!("arg{}", index), Span::call_site()))
                .collect::<Vec<_>>();

            let _async = if _async {
                quote! {.await}
            } else {
                quote! {}
            };
            let variable = quote! {
                let var = #ident( #(#arg_names),* ) #_async ;
            };


            let conditional = provider_attr
                .as_ref()
                .map(|attr| {
                    attr.conditional
                        .as_ref()
                        .map(|conditionals| {
                            quote! {
                                #[allow(unused_parens)]
                                if !( #(#conditionals())&&* )  {
                                    break 'gen;
                                }
                            }
                        })
                        .unwrap_or_default()
                })
                .unwrap_or_default();

            let conditional_on_property = conditional_on_property_attr.as_ref().map(|attr| {
                let having_value = &attr.having_value;
                let name = &attr.name;

                let generic = infer_type_from_string(having_value);
                let match_value = analyze_litstr(having_value);

                quote! {
                    match ctx.get_single_with_default_name::<ApplicationProperties>()
                        .unwrap()
                        .get_value::<#generic>(#name)
                    {
                        Some(val) => {
                            if val != #match_value {
                                break 'gen;
                            }
                        },
                        None => break 'gen
                    };
                }
            });


            let args = sig.inputs
                .iter()
                .filter_map(|input| match input {
                    FnArg::Typed(typed) => Some(typed),
                    FnArg::Receiver(_) => None,
                })
                .zip(autowired_attrs)
                .enumerate()
                .map(|(index, (input, autowired_attr))| {
                    let default_name = match input.pat.as_ref() {
                        Pat::Ident(pat) => LitStr::new(&pat.ident.to_string(), Span::call_site()),
                        _ => LitStr::new("", Span::call_site()),
                    };

                    let is_ref = match input.ty.as_ref() {
                        Type::Reference(_) => true,
                        _ => false,
                    };
                        ItemFnArgAttr {
                            ty: match input.ty.as_ref() {
                                Type::Reference(val) => val.elem.as_ref().clone(),
                                _ => input.ty.as_ref().clone()
                            },
                            arg_name: &arg_names[index],
                            default_name,
                            autowired_attr,
                            is_ref,
                        }
                })
                .map(|fn_attr| {
                    let ItemFnArgAttr {
                        ty,
                        default_name,
                        autowired_attr,
                        arg_name,
                        is_ref,
                    } = fn_attr;
                    match autowired_attr {
                        Some(attr) => {
                            let AutowiredAttr {
                                name,
                                default,
                            } = attr;

                            let name = name.unwrap_or(default_name);

                            let resolve_singleton = if is_ref {

                                if default.unwrap_or_default() {
                                    quote! { let #arg_name = ctx.resolve_option_with_name::<#ty>(#name).as_ref().unwrap_or_default(); }
                                }else {
                                    quote! { let #arg_name = ctx.resolve_with_name::<#ty>(#name).as_ref(); }
                                }
                            } else {

                                if default.unwrap_or_default() {
                                    quote! { let #arg_name = ctx.resolve_option_with_name::<#ty>(#name).unwrap_or_default(); }
                                }else {
                                    quote! { let #arg_name = ctx.resolve_with_name::<#ty>(#name); }
                                }
                            };
                            quote! {
                                 #resolve_singleton
                            }
                        }
                        None => {

                            let resolve_singleton =  if is_ref {
                                quote! { let #arg_name = ctx.resolve_with_name::<#ty>(#default_name).as_ref(); }
                            } else {
                                quote! { let #arg_name = ctx.resolve_with_name::<#ty>(#default_name); }
                            };

                            quote! { #resolve_singleton }
                        }
                    }
                });

            quote! {

                'gen: {

                    #item_fn

                    #conditional_on_property
                    #conditional

                    #(#args)*

                    #variable

                    // #insert_singleton
                }
            }
        })
        .collect::<Vec<_>>();

    Ok(impl_blocks)
}

fn supported_attributes(impl_item_fn: &ImplItemFn) -> bool {
    let attrs = ["provider", "conditional_on_property"];

    impl_item_fn
        .attrs
        .iter()
        .any(|attr| attrs.iter().any(|s| attr.path().is_ident(s)))
}

fn infer_type_from_string(lit_str: &LitStr) -> TokenStream2 {
    use std::str::FromStr;

    let value = lit_str.value();

    // 1. 检查布尔值
    if value == "true" || value == "false" {
        return quote! { bool };
    }

    // 2. 尝试解析为整数
    if let Ok(_) = i64::from_str(&value) {
        // 进一步判断应该用 i64 还是 u64 或其他
        if value.starts_with('-') {
            return quote! { i64 };
        } else {
            // 根据数值大小选择合适的无符号整数类型
            if let Ok(val) = u64::from_str(&value) {
                if val <= u8::MAX as u64 {
                    return quote! { u8 };
                } else if val <= u16::MAX as u64 {
                    return quote! { u16 };
                } else if val <= u32::MAX as u64 {
                    return quote! { u32 };
                } else {
                    return quote! { u64 };
                }
            }
        }
    }

    // 3. 尝试解析为浮点数
    if let Ok(_) = f64::from_str(&value) {
        // 检查是否包含小数点或指数表示法
        if value.contains('.') || value.contains('e') || value.contains('E') {
            return quote! { f64 };
        }
    }

    // 4. 默认返回 String
    quote! { String }
}

fn analyze_litstr(lit_str: &LitStr) -> Lit {
    let value = lit_str.value();
    let span = lit_str.span();

    // 检查布尔值
    if value == "true" {
        return Lit::Bool(LitBool::new(true, span));
    } else if value == "false" {
        return Lit::Bool(LitBool::new(false, span));
    }

    use std::str::FromStr;

    // 检查浮点数
    if value.contains('.') || value.contains('e') || value.contains('E') {
        if let Ok(_) = f64::from_str(&value) {
            return Lit::Float(LitFloat::new(&value, span));
        }
    }

    // 检查整数
    if !value.is_empty() {
        // 处理负数
        if value.starts_with('-') {
            if let Ok(val) = i64::from_str(&value) {
                return Lit::Int(LitInt::new(&val.to_string(), span));
            }
        }
        // 处理正数
        else {
            let num_part = if value.starts_with('+') {
                &value[1..]
            } else {
                &value
            };

            // 先尝试 u64
            if let Ok(val) = u64::from_str(num_part) {
                return Lit::Int(LitInt::new(&val.to_string(), span));
            }
            // 如果超出 u64 范围，尝试 i64
            else if let Ok(val) = i64::from_str(&value) {
                return Lit::Int(LitInt::new(&val.to_string(), span));
            }
        }
    }

    // 默认作为字符串
    Lit::Str(LitStr::new(&value, span))
}

struct ItemFnAtrr {
    pub ident: Ident,
    pub _async: bool,
    pub sig: Signature,
    pub provider_attr: Option<ProviderAttr>,
    pub conditional_on_property_attr: Option<ConditionalOnPropertyAttr>,
}

// pub extend_function: Vec<ImplItemFn>,
//
struct ItemFnArgAttr<'a> {
    pub ty: Type,
    pub default_name: LitStr,
    pub autowired_attr: Option<AutowiredAttr>,
    pub arg_name: &'a Ident,
    pub is_ref: bool,
}
