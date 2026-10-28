use from_attr::{AttrsValue, FromAttr, PathValue};
use next_web_context::{Color, Scope};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, ExprLit, GenericParam, Ident, ItemFn, Lit, LitStr, ReturnType};

use crate::di::{
    commons::{self, ArgumentResolveStmts},
    resource_attr::ResourceAttr,
    struct_or_function_attr::{ClosureOrPath, StructOrFunctionAttr},
};

pub(crate) fn generate(
    attr: StructOrFunctionAttr,
    mut item_fn: ItemFn,
    scope: Scope,
) -> syn::Result<TokenStream> {
    let ResourceAttr { path } = match ResourceAttr::remove_attributes(&mut item_fn.attrs) {
        Ok(Some(AttrsValue { value: attr, .. })) => attr,
        Ok(None) => ResourceAttr::default(),
        Err(AttrsValue { value: e, .. }) => return Err(e),
    };

    if let Some(PathValue { path, .. }) = attr.async_ {
        return Err(syn::Error::new(
            path,
            "`async` only support in struct and enum, please use async fn or sync fn instead",
        ));
    }

    let StructOrFunctionAttr {
        name,
        eager_create,
        condition,
        binds,
        async_: _,
        #[cfg(feature = "auto-register")]
        auto_register,
        default: _,
    } = attr;

    #[cfg(feature = "auto-register")]
    commons::check_generics_when_enable_auto_register(
        auto_register,
        &item_fn.sig.generics,
        commons::ItemKind::Function,
        scope,
    )?;

    let color = match item_fn.sig.asyncness {
        Some(_) => Color::Async,
        None => Color::Sync,
    };

    let condition = condition
        .map(|ClosureOrPath(expr)| quote!(Some(#expr)))
        .unwrap_or_else(|| quote!(None));

    let ArgumentResolveStmts {
        ref_mut_cx_stmts,
        ref_cx_stmts,
        args,
    } = commons::generate_argument_resolve_methods(&mut item_fn.sig.inputs, color)?;

    let create_provider = commons::generate_create_provider(scope, color);

    let (impl_generics, ty_generics, where_clause) = item_fn.sig.generics.split_for_impl();

    let vis = &item_fn.vis;

    let docs = item_fn
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"));

    let ident = &item_fn.sig.ident;

    // The name of the singleton a function provides defaults to the name of
    // the function in camel case, so that `fn test_name(...)` provides
    // `testName`.
    let name = provider_name(&name, ident);

    let return_type_ident = match &item_fn.sig.output {
        ReturnType::Default => quote! {
            ()
        },
        ReturnType::Type(_, ty) => quote! {
            #ty
        },
    };

    // The type the provider of the function is implemented for carries no
    // value: it only gives the provider a name of its own, and the provided
    // type is the return type of the function.
    //
    // The type has a field, even when the function is not generic, so that it
    // is not a unit struct: a field or an argument that is named like the
    // function would otherwise be read as a pattern of this type instead of as
    // a binding.
    let struct_definition = if item_fn.sig.generics.params.is_empty() {
        quote! {
            #vis struct #ident {
                _mark: (),
            }
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

    let turbofish = ty_generics.as_turbofish();
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

    #[cfg(not(feature = "auto-register"))]
    let auto_register = quote! {};

    #[cfg(feature = "auto-register")]
    let auto_register = if auto_register {
        quote! {
            #path::register_provider!(<#ident as #path::DefaultProvider>::provider());
        }
    } else {
        quote! {}
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
                        .name(#name)
                        .eager_create(#eager_create)
                        .condition(#condition)
                        #(
                            .bind(#binds)
                        )*
                )
            }
        }

        #auto_register
    };

    Ok(expand)
}

/// Returns the name the provider of a function is registered with.
///
/// The name given with `name = "..."` wins. Without it the provider is named
/// after the function, in camel case: `fn test_name(...)` provides the instance
/// named `testName`.
///
/// # Arguments
///
/// * `name` - The `name` argument of the attribute.
/// * `function` - The name of the function the provider is generated for.
fn provider_name(name: &Expr, function: &Ident) -> Expr {
    match name {
        Expr::Lit(ExprLit {
            lit: Lit::Str(lit), ..
        }) if lit.value().is_empty() => {
            let name = crate::util::name::field_name_to_singleton_name(&function.to_string());

            Expr::Lit(ExprLit {
                attrs: Vec::new(),
                lit: Lit::Str(LitStr::new(&name, lit.span())),
            })
        }
        name => name.clone(),
    }
}
