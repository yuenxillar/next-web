//! The `#[auto_configuration]` attribute.
//!
//! The attribute turns an `impl` block into a *configuration class*: every
//! method of the block that declares a provider is called while the application
//! starts, resolves the parameters of the method from the application context,
//! and registers the value the method returns as a singleton of the context.
//!
//! The configuration class itself is an
//! [`AutoConfiguration`](::next_web_core::traits::config::auto_configuration::AutoConfiguration),
//! which is the same abstraction the auto-configurations of a starter implement.
//! The generated implementation is registered as a provider of the context, so
//! the framework applies it together with every other auto-configuration of the
//! application, in the order of [`Ordered`](::next_web_core::Ordered), and only
//! when its conditions hold.

use std::collections::HashSet;

use from_attr::FromAttr;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{
    Error, Expr, ExprPath, FnArg, Ident, ImplItem, ImplItemFn, ItemImpl, LitStr, Pat, PatType,
    ReturnType, Signature, Type,
};

use crate::{util::logic::Logic, web::attrs::auto_configuration_attr::*};

/// The order a provider runs in when the configuration does not declare one,
/// which is the default order of [`Ordered`](::next_web_core::Ordered).
const DEFAULT_ORDER: &str = "100";

pub fn impl_macro_auto_configuration(attrs: TokenStream, mut item_impl: ItemImpl) -> TokenStream {
    Logic::generate(|| {
        safety_check(&item_impl)?;

        let configuration = AutoConfigurationAttr::from_tokens(attrs.into())?;

        let type_path = match item_impl.self_ty.as_ref() {
            Type::Path(type_path) => type_path.path.segments.last(),
            _ => return Err(Error::new(item_impl.self_ty.span(), "")),
        };

        let path = match type_path {
            Some(path) => path,
            None => return Err(Error::new(type_path.span(), "")),
        };

        // The type the configuration is generated for, which the conditions of
        // the configuration name with `Self`.
        let annotated = path.ident.clone();

        // The type that carries the generated implementation. Its name is
        // derived from the annotated type, so the configurations of two types do
        // not collide.
        let registration = Ident::new(&format!("__{}Gen", annotated), Span::call_site());

        // The provider that hands the configuration out is registered under the
        // name of the generated type, which keeps it apart from the provider of
        // the annotated type itself, when that type declares one as well.
        let name = match configuration.name {
            Some(name) if name.value().trim().is_empty() => {
                return Err(Error::new(name.span(), "name must not be empty"));
            }
            Some(name) => name,
            None => LitStr::new(&registration.to_string(), registration.span()),
        };

        let order: Expr = match configuration.order {
            Some(order) => order,
            None => syn::parse_str(DEFAULT_ORDER).expect("the default order is an integer"),
        };

        let conditions = configuration
            .conditional
            .into_iter()
            .map(|mut condition| {
                qualify_self(&mut condition, &annotated);
                condition
            })
            .collect::<Vec<_>>();

        let mut attributes = attributes(&mut item_impl.items)?;

        // The functions the providers name as their conditions. A condition
        // that is named without a path, `conditional = [enabled]`, is a
        // function of the module of the configuration, which is emitted with
        // the configuration so that the name resolves.
        let condition_names = attributes
            .iter_mut()
            .map(|(_index, _attrs, function)| function.provider_attr.as_mut())
            .map(|attr| {
                attr.map(|attr| attr.conditional.iter_mut())
                    .unwrap_or_default()
            })
            .flat_map(|conditions| {
                conditions
                    .map(|condition| {
                        qualify_self(condition, &annotated);
                        condition.to_token_stream().to_string()
                    })
                    .filter(|condition| !condition.contains("::"))
                    .collect::<Vec<_>>()
            })
            .collect::<HashSet<String>>()
            .into_iter()
            .map(|condition| Ident::new(&condition, Span::call_site()))
            .collect::<Vec<_>>();

        let providers = gen_code(&item_impl.items, attributes)?;

        let mut condition_functions = Vec::new();
        for item_fn in item_impl.items.iter().filter_map(|item| match item {
            ImplItem::Fn(impl_item_fn) => Some(impl_item_fn),
            _ => None,
        }) {
            if condition_names
                .iter()
                .any(|name| &item_fn.sig.ident == name)
            {
                match &item_fn.sig.output {
                    ReturnType::Default => {
                        return Err(Error::new(
                            item_fn.span(),
                            "the condition function has to declare the return type bool",
                        ));
                    }
                    ReturnType::Type(_, ty) => match ty.as_ref() {
                        Type::Path(type_path) => {
                            if !type_path.path.is_ident("bool") {
                                return Err(Error::new(
                                    item_fn.span(),
                                    "the condition function has to return a boolean value",
                                ));
                            }
                            condition_functions.push(item_fn);
                        }
                        _ => {
                            return Err(Error::new(
                                item_fn.span(),
                                "the condition function has to return a boolean value",
                            ));
                        }
                    },
                }
            }
        }

        let matches = if conditions.is_empty() {
            quote! {}
        } else {
            let conditions = conditions.iter();

            quote! {
                fn matches(&self, ctx: &dyn ::next_web_core::ApplicationContext) -> bool {
                    #[allow(unused_parens)]
                    if !( #(#conditions(ctx))&&* ) {
                        return false;
                    }

                    true
                }
            }
        };

        let expanded = quote! {

            #item_impl

            #[doc(hidden)]
            #[derive(Clone)]
            #[allow(non_camel_case_types)]
            struct #registration;

            #[::next_web_core::async_trait]
            impl ::next_web_core::traits::config::auto_configuration::AutoConfiguration
                for #registration
            {
                async fn configure(
                    &mut self,
                    ctx: &mut dyn ::next_web_core::ApplicationContext,
                ) -> ::core::result::Result<(), ::std::boxed::Box<dyn ::std::error::Error>> {
                    use ::next_web_context::ApplicationContextExt as _;

                    #(#condition_functions)*

                    #providers

                    ::core::result::Result::Ok(())
                }

                #matches
            }

            impl ::next_web_core::Ordered for #registration {
                fn order(&self) -> i32 {
                    #order
                }
            }

            // The configuration is an auto-configuration of the application,
            // so it is registered like the auto-configurations of a starter,
            // and applied by the context while it refreshes.
            ::next_web_core::register_provider!(
                <::next_web_context::Provider<
                    ::std::boxed::Box<
                        dyn ::next_web_core::traits::config::auto_configuration::AutoConfiguration
                    >
                > as ::core::convert::From<_>>::from(
                    ::next_web_context::singleton(
                        |_: &mut dyn ::next_web_core::ApplicationContext| {
                            ::std::boxed::Box::new(#registration)
                                as ::std::boxed::Box<
                                    dyn ::next_web_core::traits::config::auto_configuration::AutoConfiguration
                                >
                        }
                    )
                    .name(#name)
                )
            );
        };

        Ok(expanded)
    })
}

/// Rewrites the `Self` of a condition into the annotated type, so that a
/// condition declared as `Self::enabled` is called on the type of the
/// configuration.
///
/// # Arguments
///
/// * `condition` - The condition to rewrite.
/// * `annotated` - The type the configuration is generated for.
fn qualify_self(condition: &mut ExprPath, annotated: &Ident) {
    if let Some(first) = condition.path.segments.first_mut() {
        if first.ident == "Self" {
            first.ident = Ident::new(&annotated.to_string(), first.span());
        }
    }
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
                            "the method of an auto-configuration cannot have a self parameter",
                        ));
                    }
                    FnArg::Typed(typed) => match typed.ty.as_ref() {
                        Type::Reference(refer) => {
                            if refer.mutability.is_some() {
                                return Err(Error::new(
                                    refer.span(),
                                    format!(
                                        "the method `{}` takes a mutable parameter, which is not \
                                         allowed; a method of an auto-configuration takes its \
                                         dependencies by reference or by value",
                                        item_fn.sig.ident
                                    ),
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

/// Reads the attributes of a method of the configuration.
///
/// An attribute the method declares is an error when it cannot be read, which
/// keeps a typo in the name of an attribute from silently dropping the provider
/// it describes.
///
/// # Arguments
///
/// * `impl_item_fn` - The method the attributes are read from.
fn get_item_fn_attrs(impl_item_fn: &mut ImplItemFn) -> Result<ItemFnAttr, Error> {
    let provider_attr = ProviderAttr::remove_attributes(&mut impl_item_fn.attrs)
        .map_err(|error| error.value)?
        .map(|attribute| attribute.value);

    let conditional_on_property_attr =
        ConditionalOnPropertyAttr::remove_attributes(&mut impl_item_fn.attrs)
            .map_err(|error| error.value)?
            .map(|attribute| attribute.value);

    Ok(ItemFnAttr {
        ident: impl_item_fn.sig.ident.clone(),
        is_async: impl_item_fn.sig.asyncness.is_some(),
        signature: impl_item_fn.sig.clone(),
        provider_attr,
        conditional_on_property_attr,
    })
}

/// Reads the `#[autowired]` attribute of a parameter of a method.
///
/// # Arguments
///
/// * `pat_type` - The parameter the attribute is read from.
fn get_autowired_attr(pat_type: &mut PatType) -> Result<Option<AutowiredAttr>, Error> {
    let Some(attribute) =
        AutowiredAttr::remove_attributes(&mut pat_type.attrs).map_err(|error| error.value)?
    else {
        return Ok(None);
    };

    Ok(attribute.value.into())
}

/// Reads the attributes of every method of the configuration that declares a
/// provider.
///
/// # Arguments
///
/// * `items` - The items of the annotated implementation block.
fn attributes(
    items: &mut [ImplItem],
) -> Result<Vec<(usize, Vec<Option<AutowiredAttr>>, ItemFnAttr)>, Error> {
    let mut attributes = Vec::new();

    for (index, item_fn) in items
        .iter_mut()
        .filter_map(|item: &mut ImplItem| match item {
            ImplItem::Fn(impl_item_fn) => Some(impl_item_fn),
            _ => None,
        })
        .filter(|item| !matches!(item.sig.output, ReturnType::Default))
        .filter(|item_fn| supported_attributes(item_fn))
        .enumerate()
    {
        item_fn
            .attrs
            .push(syn::parse_quote! { #[allow(dead_code)] });

        let mut autowired = Vec::new();
        for input in item_fn.sig.inputs.iter_mut() {
            if let FnArg::Typed(typed) = input {
                autowired.push(get_autowired_attr(typed)?);
            }
        }

        attributes.push((index, autowired, get_item_fn_attrs(item_fn)?));
    }

    Ok(attributes)
}

/// Expands the methods of the configuration into the calls that create their
/// instances.
///
/// Every method becomes a function of its own, and the functions are collected
/// with the order their provider declares, sorted, and called in that order.
/// The order is an expression, so a provider can be given the order of a
/// constant, and the providers of the same order keep the order they were
/// declared in.
///
/// # Arguments
///
/// * `items` - The items of the annotated implementation block.
/// * `attributes` - The attributes of the methods that declare a provider.
fn gen_code(
    items: &[ImplItem],
    attributes: Vec<(usize, Vec<Option<AutowiredAttr>>, ItemFnAttr)>,
) -> Result<TokenStream2, Error> {
    let providers = items
        .iter()
        .filter_map(|item| match item {
            ImplItem::Fn(impl_item_fn) => Some(impl_item_fn),
            _ => None,
        })
        .filter(|item| !matches!(item.sig.output, ReturnType::Default))
        .zip(attributes)
        .enumerate()
        .filter(|(index, (_item_fn, (i, _, _)))| index == i)
        .map(|(index, (item_fn, (_, autowired_attrs, function)))| {
            expand_provider(index, item_fn, autowired_attrs, function)
        })
        .collect::<Result<Vec<_>, Error>>()?;

    let indices = providers
        .iter()
        .map(|provider| provider.index)
        .collect::<Vec<_>>();
    let pushed_indices = indices.iter();
    let matched_indices = indices.iter();
    let orders = providers.iter().map(|provider| &provider.order);
    let bodies = providers.iter().map(|provider| &provider.body);

    Ok(quote! {
        // The providers are collected with the order they declare, so that the
        // order of a provider can be the value of a constant of the
        // application, and they run from the lowest order to the highest one.
        let mut __providers: ::std::vec::Vec<(i32, usize)> = ::std::vec::Vec::new();

        #(
            __providers.push((#orders, #pushed_indices));
        )*

        __providers.sort_by_key(|(order, _)| *order);

        for (_, __provider) in __providers {
            match __provider {
                #(
                    #matched_indices => {
                        #bodies
                    }
                )*
                _ => {}
            }
        }
    })
}

/// A provider of an auto-configuration, which is the block that creates the
/// instance of the provider and the order it runs in.
struct Provider {
    /// The position of the provider in the configuration, which selects the
    /// block that creates the instance.
    index: usize,
    /// The block that creates the instance.
    body: TokenStream2,
    /// The expression that yields the order the provider runs in.
    order: Expr,
}

/// Expands one method of the configuration into the function that creates its
/// instance.
///
/// # Arguments
///
/// * `index` - The position of the method in the implementation block, which
///   names the generated function.
/// * `item_fn` - The method that declares the provider.
/// * `autowired_attrs` - The `#[autowired]` attributes of the parameters.
/// * `function` - The attributes of the method.
fn expand_provider(
    index: usize,
    item_fn: &ImplItemFn,
    autowired_attrs: Vec<Option<AutowiredAttr>>,
    function: ItemFnAttr,
) -> Result<Provider, Error> {
    let ItemFnAttr {
        ident,
        is_async,
        signature: sig,
        mut provider_attr,
        conditional_on_property_attr,
    } = function;

    let arg_names = (0..sig.inputs.len())
        .map(|index| Ident::new(&format!("arg{index}"), Span::call_site()))
        .collect::<Vec<_>>();

    let is_async = if is_async {
        quote! { .await }
    } else {
        quote! {}
    };

    let order = provider_order(&provider_attr)?;

    let conditional = provider_attr
        .as_ref()
        .map(|attr| {
            if attr.conditional.is_empty() {
                return quote! {};
            }
            let conditional = attr.conditional.iter();
            quote! {
                #[allow(unused_parens)]
                if !( #(#conditional(ctx))&&* )  {
                    break 'gen;
                }
            }
        })
        .unwrap_or_default();

    let conditional_on_property = conditional_on_property_attr.as_ref().map(|attr| {
        let name = &attr.name;
        let having_value = &attr.having_value;

        // The property is read from the environment of the
        // application, which is where the configuration file, the
        // environment variables and the command line arguments of
        // the process meet, and its value is compared as the text
        // the environment resolved for it.
        quote! {
            let __property = match ctx
                .get_singleton_option_with_name::<
                    ::std::sync::Arc<
                        dyn ::next_web_core::env::ConfigurableEnvironment
                    >
                >(::next_web_context::APPLICATION_ENVIRONMENT_SINGLETON_NAME)
            {
                ::core::option::Option::Some(__environment) => {
                    use ::next_web_core::env::PropertyResolver as _;

                    __environment.get_property(#name)
                }
                ::core::option::Option::None => ::core::option::Option::None,
            };

            if __property.as_deref() != ::core::option::Option::Some(#having_value) {
                break 'gen;
            }
        }
    });

    let args = sig
        .inputs
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

            let is_ref = matches!(input.ty.as_ref(), Type::Reference(_));

            ItemFnArgAttr {
                ty: match input.ty.as_ref() {
                    Type::Reference(val) => val.elem.as_ref().clone(),
                    _ => input.ty.as_ref().clone(),
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

            let (name, default) = match autowired_attr {
                Some(AutowiredAttr { name, default }) => {
                    (name.unwrap_or(default_name), default.unwrap_or_default())
                }
                None => (default_name, false),
            };

            let resolve = match (is_ref, default) {
                (true, true) => quote! {
                    let #arg_name =
                        &ctx.resolve_option_with_name::<#ty>(#name).unwrap_or_default();
                },
                (true, false) => quote! {
                    let #arg_name = &ctx.resolve_with_name::<#ty>(#name);
                },
                (false, true) => quote! {
                    let #arg_name =
                        ctx.resolve_option_with_name::<#ty>(#name).unwrap_or_default();
                },
                (false, false) => quote! {
                    let #arg_name = ctx.resolve_with_name::<#ty>(#name);
                },
            };

            quote! { #resolve }
        });

    let result_unwrap = provider_result_unwrap(&sig);
    let instance = quote! {
        let instance = #ident( #(#arg_names),* ) #is_async #result_unwrap;
    };

    let singleton_name = provider_attr
        .as_mut()
        .map(|attr| attr.name.take())
        .unwrap_or_default()
        .unwrap_or_else(|| LitStr::new(&item_fn.sig.ident.to_string(), Span::call_site()));
    let register_singleton = quote! { ctx.insert_singleton_with_name(instance, #singleton_name); };

    Ok(Provider {
        index,
        order,
        body: quote! {
            'gen: {

                #item_fn

                #conditional_on_property
                #conditional

                #(#args)*

                #instance
                #register_singleton
            }
        },
    })
}

/// Returns the order a provider runs in.
///
/// # Arguments
///
/// * `provider_attr` - The attributes of the provider.
fn provider_order(provider_attr: &Option<ProviderAttr>) -> Result<Expr, Error> {
    match provider_attr.as_ref().and_then(|attr| attr.order.as_ref()) {
        Some(order) => Ok(order.clone()),
        None => syn::parse_str(DEFAULT_ORDER).map_err(|_| {
            Error::new(
                Span::call_site(),
                "the default order of a provider is a whole number",
            )
        }),
    }
}

fn extract_result_ok_type(ty: &Type) -> Option<Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };

    let segment = type_path.path.segments.last()?;
    if segment.ident != "Result" {
        return None;
    }

    let syn::PathArguments::AngleBracketed(generic_args) = &segment.arguments else {
        return None;
    };

    generic_args.args.iter().find_map(|arg| match arg {
        syn::GenericArgument::Type(ty) => Some(ty.clone()),
        _ => None,
    })
}

fn provider_result_unwrap(sig: &Signature) -> TokenStream2 {
    match &sig.output {
        ReturnType::Type(_, ty) if extract_result_ok_type(ty.as_ref()).is_some() => quote! { ? },
        _ => quote! {},
    }
}

fn supported_attributes(impl_item_fn: &ImplItemFn) -> bool {
    let attrs = ["provider", "conditional_on_property"];

    impl_item_fn
        .attrs
        .iter()
        .any(|attr| attrs.iter().any(|s| attr.path().is_ident(s)))
}

struct ItemFnAttr {
    pub ident: Ident,
    pub is_async: bool,
    pub signature: Signature,
    pub provider_attr: Option<ProviderAttr>,
    pub conditional_on_property_attr: Option<ConditionalOnPropertyAttr>,
}

struct ItemFnArgAttr<'a> {
    pub ty: Type,
    pub default_name: LitStr,
    pub autowired_attr: Option<AutowiredAttr>,
    pub arg_name: &'a Ident,
    pub is_ref: bool,
}

#[cfg(test)]
mod tests {
    use from_attr::FromAttr;
    use quote::ToTokens;
    use syn::parse_str;

    use super::{extract_result_ok_type, provider_result_unwrap};
    use crate::web::attrs::auto_configuration_attr::{AutoConfigurationAttr, ProviderAttr};

    #[test]
    fn the_attributes_of_a_provider_are_read() {
        let attributes = ProviderAttr::from_tokens(quote::quote! {
            name = "value", conditional = [Self::enabled], order = 12
        })
        .unwrap();

        assert_eq!(
            attributes.name.map(|name| name.value()),
            Some(String::from("value"))
        );
        assert_eq!(attributes.conditional.len(), 1);
        assert_eq!(
            attributes
                .order
                .map(|order| order.to_token_stream().to_string()),
            Some(String::from("12"))
        );
    }

    #[test]
    fn the_order_of_a_provider_is_an_expression() {
        let attributes = ProviderAttr::from_tokens(quote::quote! {
            order = i32::MIN
        })
        .unwrap();

        assert_eq!(
            attributes
                .order
                .map(|order| order.to_token_stream().to_string()),
            Some(String::from("i32 :: MIN"))
        );
    }

    #[test]
    fn the_attributes_of_a_configuration_are_read() {
        let attributes = AutoConfigurationAttr::from_tokens(quote::quote! {
            name = "testAutoConfiguration",
            order = 20,
            conditional = [Self::enabled, enabled_in_the_module]
        })
        .unwrap();

        assert_eq!(
            attributes.name.map(|name| name.value()),
            Some(String::from("testAutoConfiguration"))
        );
        assert_eq!(
            attributes
                .order
                .map(|order| order.to_token_stream().to_string()),
            Some(String::from("20"))
        );
        assert_eq!(attributes.conditional.len(), 2);
    }

    #[test]
    fn a_configuration_without_attributes_is_read() {
        let attributes = AutoConfigurationAttr::from_tokens(quote::quote! {}).unwrap();

        assert!(attributes.name.is_none());
        assert!(attributes.order.is_none());
        assert!(attributes.conditional.is_empty());
    }

    #[test]
    fn a_value_of_the_wrong_type_is_reported() {
        let attributes = ProviderAttr::from_tokens(quote::quote! {
            name = 12
        });

        assert!(attributes.is_err());
    }

    #[test]
    fn extract_result_ok_type_returns_inner_type() {
        let ty = parse_str::<syn::Type>("std::result::Result<String, BoxError>").unwrap();
        let ok_ty = extract_result_ok_type(&ty).unwrap();

        assert_eq!(ok_ty.to_token_stream().to_string(), "String");
    }

    #[test]
    fn provider_result_unwrap_adds_question_mark_for_result() {
        let sig =
            parse_str::<syn::Signature>("fn redis_template() -> Result<String, BoxError>").unwrap();

        assert_eq!(provider_result_unwrap(&sig).to_string(), "?");
    }

    #[test]
    fn provider_result_unwrap_ignores_plain_return_type() {
        let sig = parse_str::<syn::Signature>("fn redis_template() -> String").unwrap();

        assert!(provider_result_unwrap(&sig).is_empty());
    }
}
