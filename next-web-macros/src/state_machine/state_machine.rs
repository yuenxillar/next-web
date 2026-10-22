//! The `#[state_machine]` attribute macro.
//!
//! It sits on the `impl StateMachineConfigurer<S, E>` block of an application and
//! makes the machine the configuration describes part of the application
//! context, so a component injects it:
//!
//! ```ignore
//! #[state_machine(name = "orderStateMachine")]
//! impl StateMachineConfigurer<OrderState, OrderEvent> for OrderStateMachineConfig { .. }
//! ```
//!
//! The configuration has to be a unit struct which derives `Clone`: the macro
//! submits an instance of it to the inventory, and a unit struct is the value the
//! inventory can hold.

use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{Expr, ExprLit, ItemImpl, Lit, Type, parse_macro_input};

/// The arguments of the attribute.
#[derive(Default)]
struct Arguments {
    name: Option<String>,
}

/// Wraps an `impl StateMachineConfigurer` block.
///
/// # Arguments
/// * `name = ".."` - the bean name of the machine, `stateMachine` by default
pub fn impl_macro_state_machine(attr: TokenStream, item: TokenStream) -> TokenStream {
    let block = parse_macro_input!(item as ItemImpl);

    match expand(&block, attr) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Expands the block into itself, the registration impl and the submission.
fn expand(block: &ItemImpl, attr: TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let arguments = arguments(attr)?;
    let name = arguments
        .name
        .unwrap_or_else(|| String::from("stateMachine"));
    let (state, event) = configurer_types(block)?;
    let self_ty = &block.self_ty;

    let configurer_name = self_ty.to_token_stream().to_string().replace(' ', "");

    Ok(quote! {
        #block

        impl ::next_web::autoregister::state_machine_autoregister::StateMachineAutoRegister
            for #self_ty
        {
            fn configurer_name(&self) -> &'static str {
                #configurer_name
            }

            fn machine_name(&self) -> &'static str {
                #name
            }

            fn register(
                &self,
                context: &mut dyn ::next_web::core::ApplicationContext,
            ) -> ::std::result::Result<(), ::next_web::core::error::BoxError> {
                ::next_web::autoregister::state_machine_autoregister::register_machine::<
                    #state,
                    #event,
                    Self,
                >(context, ::core::clone::Clone::clone(self), #name)
            }
        }

        ::next_web::inventory::submit! {
            &#self_ty as &dyn
                ::next_web::autoregister::state_machine_autoregister::StateMachineAutoRegister
        }
    })
}

/// Parses the `name` argument of the attribute.
fn arguments(attr: TokenStream) -> syn::Result<Arguments> {
    let parser = syn::punctuated::Punctuated::<syn::MetaNameValue, syn::Token![,]>::parse_terminated;
    let entries = syn::parse::Parser::parse(parser, attr)?;

    let mut arguments = Arguments::default();

    for entry in entries {
        if !entry.path.is_ident("name") {
            return Err(syn::Error::new_spanned(
                &entry.path,
                "unknown argument, expected `name = \"..\"`",
            ));
        }

        let Expr::Lit(ExprLit {
            lit: Lit::Str(text),
            ..
        }) = entry.value
        else {
            return Err(syn::Error::new_spanned(
                entry.value,
                "the name has to be a string literal",
            ));
        };

        arguments.name = Some(text.value());
    }

    Ok(arguments)
}

/// Extracts the state and the event type of the configuration.
fn configurer_types(block: &ItemImpl) -> syn::Result<(Type, Type)> {
    let Some((_, path, _)) = &block.trait_ else {
        return Err(syn::Error::new_spanned(
            block,
            "the attribute belongs on `impl StateMachineConfigurer<S, E> for ..`",
        ));
    };

    let Some(segment) = path.segments.last() else {
        return Err(syn::Error::new_spanned(
            path,
            "the attribute belongs on `impl StateMachineConfigurer<S, E> for ..`",
        ));
    };

    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err(syn::Error::new_spanned(
            &segment.arguments,
            "the configuration has to name its state and event types",
        ));
    };

    let mut types = arguments.args.iter().filter_map(|argument| match argument {
        syn::GenericArgument::Type(ty) => Some(ty.clone()),
        _ => None,
    });

    let Some(state) = types.next() else {
        return Err(syn::Error::new_spanned(
            &segment.arguments,
            "the configuration is missing its state type",
        ));
    };

    let Some(event) = types.next() else {
        return Err(syn::Error::new_spanned(
            &segment.arguments,
            "the configuration is missing its event type",
        ));
    };

    Ok((state, event))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_types_of_the_configurer_are_read() {
        let block: ItemImpl = syn::parse_quote! {
            impl StateMachineConfigurer<OrderState, OrderEvent> for OrderStateMachineConfig {
                fn config_configure(&mut self) -> Result<(), BoxError> {
                    Ok(())
                }
            }
        };

        let (state, event) = configurer_types(&block).expect("the types are read");

        assert!(state.to_token_stream().to_string().contains("OrderState"));
        assert!(event.to_token_stream().to_string().contains("OrderEvent"));
    }

    #[test]
    fn an_impl_without_the_trait_is_rejected() {
        let block: ItemImpl = syn::parse_quote! {
            impl OrderStateMachineConfig {
                fn other(&self) {}
            }
        };

        assert!(configurer_types(&block).is_err());
    }
}
