use from_attr::{AttrsValue, FromAttr, PathValue};
use proc_macro2::TokenStream;
use quote::quote;
use rudi_core::{Color, Scope};
use syn::ItemStruct;

use crate::{
    commons::{self, FieldResolveStmts, ResolvedFields},
    autowired_attr::AutowiredAttr,
    struct_or_function_attr::{ClosureOrPath, StructOrFunctionAttr},
};

pub(crate) fn generate(
    attr: StructOrFunctionAttr,
    mut item_struct: ItemStruct,
    scope: Scope,
) -> syn::Result<TokenStream> {
    let AutowiredAttr { path } = match AutowiredAttr::remove_attributes(&mut item_struct.attrs) {
        Ok(Some(AttrsValue { value: attr, .. })) => attr,
        Ok(None) => AutowiredAttr::default(),
        Err(AttrsValue { value: e, .. }) => return Err(e),
    };

    let StructOrFunctionAttr {
        name,
        eager_create,
        condition,
        binds,
        async_,
        #[cfg(feature = "auto-register")]
        auto_register,
        default
    } = attr;

    #[cfg(feature = "auto-register")]
    commons::check_generics_when_enable_auto_register(
        auto_register,
        &item_struct.generics,
        commons::ItemKind::Struct,
        scope,
    )?;

    let color = match async_ {
        Some(PathValue { value: true, .. }) => Color::Async,
        _ => Color::Sync,
    };

    let condition = condition
        .map(|ClosureOrPath(expr)| quote!(Some(#expr)))
        .unwrap_or_else(|| quote!(None));

    let FieldResolveStmts {
        ref_mut_cx_stmts,
        ref_cx_stmts,
        fields,
    } = commons::generate_field_resolve_stmts(&mut item_struct.fields, color)?;

    let create_provider = commons::generate_create_provider(scope, color);

    let struct_ident = &item_struct.ident;

    let (impl_generics, ty_generics, where_clause) = item_struct.generics.split_for_impl();

    let instance = match fields {
        ResolvedFields::Unit => quote! {
            #struct_ident
        },
        ResolvedFields::Named {
            field_names,
            field_values,
        } => {
            quote! {
                #struct_ident {
                    #(
                        #field_names: #field_values,
                    )*
                }
            }
        }
        ResolvedFields::Unnamed(field_values) => {
            quote! {
                #struct_ident(
                    #(
                        #field_values,
                    )*
                )
            }
        }
    };

    
    let constructor = match color {
        Color::Async => {
            if default {
                quote! {
                    #[allow(unused_variables)]
                    |cx| {
                        #struct_ident::default()
                    }
                }
            }else {
                quote! {
                    #[allow(unused_variables)]
                    |cx| ::std::boxed::Box::pin(async {
                        #(#ref_mut_cx_stmts)*
                        #(#ref_cx_stmts)*
                        #instance
                    })
                }
                
            }
            
        }
        Color::Sync => {
            if default {
                quote! {
                    #[allow(unused_variables)]
                    |cx| {
                        #struct_ident::default()
                    }
                }
            }
            else {
                quote! {
                    #[allow(unused_variables)]
                    |cx| {
                        #(#ref_mut_cx_stmts)*
                        #(#ref_cx_stmts)*
                        #instance
                    }
                }

            }
            
        }
    };
    println!("constructor: {}", constructor.to_string());


    #[cfg(not(feature = "auto-register"))]
    let auto_register = quote! {};


    #[cfg(feature = "auto-register")]
    let auto_register = if auto_register {
        quote! {
            #path::register_provider!(<#struct_ident as #path::DefaultProvider>::provider());
        }
    } else {
        quote! {}
    };

    let expand = quote! {
        #item_struct

        impl #impl_generics #path::DefaultProvider for #struct_ident #ty_generics #where_clause {
            type Type = Self;

            fn provider() -> #path::Provider<Self> {
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

    println!("expand11: {}\n", expand.to_string());

    Ok(expand)
}
