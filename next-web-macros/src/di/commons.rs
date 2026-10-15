use from_attr::{AttrsValue, FlagOrValue, FromAttr};
use next_web_context::{Color, Scope};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse_quote, punctuated::Punctuated, spanned::Spanned, AngleBracketedGenericArguments,
    Attribute, Expr, Field, Fields, FieldsNamed, FieldsUnnamed, FnArg, GenericArgument, Ident, Pat,
    PatIdent, PatType, Path, PathArguments, PathSegment, Stmt, Token, Type, TypePath,
    TypeReference,
};

use crate::di::{field_or_argument_attr::FieldOrArgumentAttr, value_attr::ValueAttr};

/// The import that the generated statements need in order to call the generic
/// methods of the application context.
///
/// The context interface is object safe, so its generic client methods live in
/// an extension trait that has to be imported by the code that calls them.
fn context_ext_import() -> Stmt {
    parse_quote! { use ::next_web_context::ApplicationContextExt as _; }
}

/// The import that the generated statements need in order to call the
/// asynchronous methods of the application context.
///
/// The asynchronous client methods live in an extension trait of their own, so
/// a statement that awaits the constructor of a provider has to import it as
/// well.
fn async_context_ext_import() -> Stmt {
    parse_quote! { use ::next_web_context::ApplicationContextAsyncExt as _; }
}

/// Adds the imports the generated statements need in front of them.
///
/// A statement resolves an instance through the context, so it needs the
/// extension trait that provides the client methods, and the asynchronous ones
/// need the trait that awaits the constructors of the providers.
///
/// # Arguments
///
/// * `stmts` - The statements that need the context.
/// * `color` - Whether the statements were generated for an asynchronous
///   constructor.
fn with_context_imports(stmts: Vec<Stmt>, color: Color) -> Vec<Stmt> {
    if stmts.is_empty() {
        return stmts;
    }

    let mut imports = vec![context_ext_import()];

    if color == Color::Async {
        imports.push(async_context_ext_import());
    }

    imports.extend(stmts);
    imports
}

pub(crate) fn generate_create_provider(scope: Scope, color: Color) -> TokenStream {
    match (scope, color) {
        (Scope::Singleton, Color::Async) => quote! {
            singleton_async
        },
        (Scope::Singleton, Color::Sync) => quote! {
            singleton
        },
        (Scope::Transient, Color::Async) => quote! {
            transient_async
        },
        (Scope::Transient, Color::Sync) => quote! {
            transient
        },
        (Scope::SingleOwner, Color::Async) => quote! {
            single_owner_async
        },
        (Scope::SingleOwner, Color::Sync) => quote! {
            single_owner
        },
    }
}

fn extract_ref_type(ty: &Type) -> syn::Result<&Type> {
    fn require_type_ref(ty: &Type) -> Option<&TypeReference> {
        match ty {
            Type::Reference(type_ref) => Some(type_ref),
            _ => None,
        }
    }

    fn get_type_from_ref(
        TypeReference {
            mutability, elem, ..
        }: &TypeReference,
    ) -> syn::Result<&Type> {
        if mutability.is_some() {
            Err(syn::Error::new(
                mutability.span(),
                "not support mutable reference",
            ))
        } else {
            Ok(elem)
        }
    }

    let mut ty: &Type = match require_type_ref(ty) {
        Some(type_ref) => get_type_from_ref(type_ref)?,
        None => {
            return Err(syn::Error::new(
                ty.span(),
                "not support non-reference type, \
        please change to a reference type, \
        or if using a type alias, specify the original type using `#[autowired(ref = T)]`, \
        where `T` is a non-reference type",
            ));
        }
    };

    loop {
        ty = match require_type_ref(ty) {
            Some(type_ref) => get_type_from_ref(type_ref)?,
            None => break,
        };
    }

    Ok(ty)
}

/// Returns the type below every reference of `ty`.
///
/// The type of the instance a reference stands for is the one below its
/// references, so `&T`, `&mut T` and `&&T` all yield `T`.
///
/// # Arguments
///
/// * `ty` - The type the references are removed from.
fn strip_references(ty: &Type) -> &Type {
    let mut ty = ty;

    while let Type::Reference(TypeReference { elem, .. }) = ty {
        ty = elem;
    }

    ty
}

fn extract_path_type<'a>(ty: &'a Type, ty_name: &str) -> syn::Result<&'a Type> {
    let Type::Path(TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments,
        },
    }) = ty
    else {
        return Err(syn::Error::new(
            ty.span(),
            format!("only support `{}<T>` type", ty_name),
        ));
    };

    let Some(segment) = segments.last() else {
        return Err(syn::Error::new(
            ty.span(),
            "not support path type with empty segments",
        ));
    };

    let PathSegment {
        ident,
        arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }),
    } = segment
    else {
        return Err(syn::Error::new(
            segment.span(),
            "only support angle bracketed generic argument",
        ));
    };

    if ident != ty_name {
        return Err(syn::Error::new(
            ident.span(),
            format!("only support `{}<T>` type", ty_name),
        ));
    }

    let Some(arg) = args.first() else {
        return Err(syn::Error::new(
            segment.span(),
            format!(
                "not support `{}<T>` type with empty generic arguments ",
                ty_name
            ),
        ));
    };

    if args.len() > 1 {
        let msg = format!(
            "only support `{}<T>` type with one generic argument",
            ty_name
        );

        if let Some(e) = args
            .iter()
            .skip(1)
            .map(|arg| syn::Error::new(arg.span(), &msg))
            .reduce(|mut a, b| {
                a.combine(b);
                a
            })
        {
            return Err(e);
        }
    }

    if let GenericArgument::Type(ty) = arg {
        extract_ref_type(ty)
    } else {
        Err(syn::Error::new(
            arg.span(),
            "only support generic argument type",
        ))
    }
}

fn extract_path_type_with_owned_type(ty: &Type, ty_name: &str) -> syn::Result<Type> {
    let Type::Path(TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments,
        },
    }) = ty
    else {
        return Err(syn::Error::new(
            ty.span(),
            format!("only support `{}<T>` type", ty_name),
        ));
    };

    let Some(segment) = segments.last() else {
        return Err(syn::Error::new(
            ty.span(),
            "not support path type with empty segments",
        ));
    };

    let PathSegment {
        ident,
        arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }),
    } = segment
    else {
        return Err(syn::Error::new(
            segment.span(),
            "only support angle bracketed generic argument",
        ));
    };

    if ident != ty_name {
        return Err(syn::Error::new(
            ident.span(),
            format!("only support `{}<T>` type", ty_name),
        ));
    }

    let Some(arg) = args.first() else {
        return Err(syn::Error::new(
            segment.span(),
            format!(
                "not support `{}<T>` type with empty generic arguments ",
                ty_name
            ),
        ));
    };

    if args.len() > 1 {
        if ty_name == "HashMap" || ty_name == "BTreeMap" {
            let last_args = args.last().unwrap();
            if let GenericArgument::Type(last_ty) = last_args {
                let Type::Path(TypePath {
                    qself: None,
                    path:
                        Path {
                            leading_colon: None,
                            segments,
                        },
                }) = last_ty
                else {
                    return Err(syn::Error::new(
                        last_ty.span(),
                        format!("only support `{}<T>` type", ty_name),
                    ));
                };

                let val = segments.last().unwrap();
                let mut pun = Punctuated::new();
                pun.push(val.to_owned());
                return syn::Result::Ok(Type::Path(TypePath {
                    qself: None,
                    path: Path {
                        leading_colon: None,
                        segments: pun,
                    },
                }));
            }
        }
    }

    return Err(syn::Error::new(
        arg.span(),
        "only support generic argument type",
    ));
}

fn extract_option_type(ty: &Type) -> syn::Result<&Type> {
    extract_path_type(ty, "Option")
}

fn extract_vec_type(ty: &Type) -> syn::Result<&Type> {
    extract_path_type(ty, "Vec")
}

/// The value type of a map of instances.
///
/// Which map a field or an argument is declared as is the caller's choice, so
/// both `HashMap` and `BTreeMap` are accepted: the macro only resolves the values
/// and lets the declared type collect them.
fn extract_hashmap_type(ty: &Type) -> syn::Result<Type> {
    extract_path_type_with_owned_type(ty, "HashMap")
        .or_else(|_| extract_path_type_with_owned_type(ty, "BTreeMap"))
}

/// The statement that resolves one field or argument of an item, which is
/// either a value the context hands out or a reference the context lends.
///
/// A reference cannot be lent before the instance exists, so it carries the
/// statement that creates the instance as well: the two statements end up in
/// the two sets of statements an item runs before it is built.
pub(crate) enum ResolveOneValue {
    /// The statement that resolves an owned value.
    Owned { resolve: Stmt },
    /// The statement that creates the instance, and the statement that borrows
    /// it from the context.
    Ref {
        create_single: Stmt,
        get_singleton: Stmt,
    },
}

struct ResolveOne {
    stmt: ResolveOneValue,
    variable: Ident,
}

fn generate_only_one_field_or_argument_resolve_stmt(
    field_name: Option<Ident>,
    attrs: &mut Vec<Attribute>,
    color: Color,
    index: usize,
    field_or_argument_ty: &Type,
) -> syn::Result<ResolveOne> {
    let FieldOrArgumentAttr {
        name,
        option,
        default,
        vec,
        map,
        ref_,
    } = match FieldOrArgumentAttr::remove_attributes(attrs) {
        Ok(Some(AttrsValue { value, .. })) => value,
        Ok(None) => FieldOrArgumentAttr::default(),
        Err(AttrsValue { value, .. }) => return Err(value),
    };

    // A field or an argument that is declared as a reference does not have to
    // spell out `ref`: whether the instance is taken from the context mutably
    // or immutably is already written in the type. The value is `Some(true)`
    // for a `&mut T`, `Some(false)` for a `&T` and `None` when the type is not
    // a reference, or when `ref` was given explicitly.
    let inferred_ref = match ref_ {
        FlagOrValue::None => match field_or_argument_ty {
            Type::Reference(TypeReference { mutability, .. }) => Some(mutability.is_some()),
            _ => None,
        },
        FlagOrValue::Flag { .. } | FlagOrValue::Value { .. } => None,
    };

    // If name is empty, It is to generate singleton names using field names
    let name: Expr = match name {
        Expr::Lit(expr_lit) => {
            let signleton_name = expr_lit.lit.to_token_stream().to_string().replace("\"", "");
            if signleton_name.is_empty() {
                match field_name {
                    Some(_name) => {
                        // A leading underscore only marks a name that is not
                        // used, so it is not part of the name of the instance.
                        let field_name = _name.to_string();
                        let field_name = field_name.trim_start_matches('_');
                        let singleton_name =
                            crate::util::name::field_name_to_singleton_name(field_name);

                        Expr::Lit(syn::PatLit {
                            attrs: Default::default(),
                            lit: syn::Lit::Str(syn::LitStr::new(
                                &singleton_name,
                                Span::call_site(),
                            )),
                        })
                    }
                    None => {
                        // A reference stands for the type it refers to, so a
                        // field or an argument is named after the type below
                        // its references, with the first character lower cased:
                        // `&MyService` is named `myService`. A type with
                        // generic arguments is named after its own name, e.g.
                        // `Vec<MyService>` after `Vec`.
                        match strip_references(field_or_argument_ty) {
                            syn::Type::Path(TypePath { qself: None, path }) => {
                                match path.segments.last() {
                                    Some(segment) => {
                                        let name = segment.ident.to_string();
                                        let singleton_name =
                                            crate::util::name::singleton_name(&name);

                                        Expr::Lit(syn::PatLit {
                                            attrs: Default::default(),
                                            lit: syn::Lit::Str(syn::LitStr::new(
                                                &singleton_name,
                                                Span::call_site(),
                                            )),
                                        })
                                    }
                                    None => Expr::Lit(expr_lit),
                                }
                            }
                            _ => Expr::Lit(expr_lit),
                        }
                    }
                }
            } else {
                Expr::Lit(expr_lit)
            }
        }
        _ => name,
    };

    // macro processing for properties
    let ValueAttr {
        key,
        default: value_default,
    } = match ValueAttr::remove_attributes(attrs) {
        Ok(Some(AttrsValue { value: attr, .. })) => attr,
        Ok(None) => ValueAttr::default(),
        Err(AttrsValue { value, .. }) => return Err(value),
    };

    let var1 = key.clone().to_token_stream().to_string();
    let is_value = if !var1.replace("\"", "").is_empty() {
        true
    } else {
        false
    };

    // A reference is bound to a name of its own, whether it was spelled out
    // with `ref` or inferred from the type of the field or argument.
    let is_reference = inferred_ref.is_some() || !matches!(ref_, FlagOrValue::None);
    let ident = match is_reference {
        false => format_ident!("owned_{}", index),
        true => format_ident!("ref_{}", index),
    };

    if option {
        let ty = match ref_ {
            FlagOrValue::None => None,
            FlagOrValue::Flag { .. } => {
                let ty = extract_option_type(field_or_argument_ty)?;
                Some(quote!(#ty))
            }
            FlagOrValue::Value { value: ty, .. } => Some(quote!(#ty)),
        };

        return match ty {
            Some(ty) => {
                let create_single = match color {
                    Color::Async => parse_quote! {
                        cx.try_just_create_singleton_with_name_async::<#ty>(#name).await;
                    },
                    Color::Sync => parse_quote! {
                        cx.try_just_create_singleton_with_name::<#ty>(#name);
                    },
                };

                let get_singleton = parse_quote! {
                    let #ident = cx.get_singleton_option_with_name(#name);
                };

                Ok(ResolveOne {
                    stmt: ResolveOneValue::Ref {
                        create_single,
                        get_singleton,
                    },
                    variable: ident,
                })
            }
            None => {
                let resolve = match color {
                    Color::Async => parse_quote! {
                        let #ident = cx.resolve_option_with_name_async(#name).await;
                    },
                    Color::Sync => parse_quote! {
                        let #ident = cx.resolve_option_with_name(#name);
                    },
                };

                Ok(ResolveOne {
                    stmt: ResolveOneValue::Owned { resolve },
                    variable: ident,
                })
            }
        };
    }

    let default = match default {
        FlagOrValue::None => None,
        FlagOrValue::Flag { .. } => Some(parse_quote!(::core::default::Default::default())),
        FlagOrValue::Value { value: expr, .. } => Some(expr),
    };

    if let Some(default) = default {
        let ty = match ref_ {
            FlagOrValue::None => None,
            FlagOrValue::Flag { .. } => {
                let ty = extract_ref_type(field_or_argument_ty)?;
                Some(quote!(#ty))
            }
            FlagOrValue::Value { value: ty, .. } => Some(quote!(#ty)),
        };

        return match ty {
            Some(ty) => {
                let create_single = match color {
                    Color::Async => parse_quote! {
                        cx.try_just_create_singleton_with_name_async::<#ty>(#name).await;
                    },
                    Color::Sync => parse_quote! {
                        cx.try_just_create_singleton_with_name::<#ty>(#name);
                    },
                };

                let get_singleton = parse_quote! {
                    let #ident = match cx.get_singleton_option_with_name(#name) {
                        Some(value) => value,
                        None => #default,
                    };
                };

                Ok(ResolveOne {
                    stmt: ResolveOneValue::Ref {
                        create_single,
                        get_singleton,
                    },
                    variable: ident,
                })
            }
            None => {
                let resolve = match color {
                    Color::Async => parse_quote! {
                        let #ident = match cx.resolve_option_with_name_async(#name).await {
                            Some(value) => value,
                            None => #default,
                        };
                    },
                    Color::Sync => parse_quote! {
                        let #ident = match cx.resolve_option_with_name(#name) {
                            Some(value) => value,
                            None => #default,
                        };
                    },
                };

                Ok(ResolveOne {
                    stmt: ResolveOneValue::Owned { resolve },
                    variable: ident,
                })
            }
        };
    }

    if vec {
        let ty = match ref_ {
            FlagOrValue::None => None,
            FlagOrValue::Flag { .. } => {
                let ty = extract_vec_type(field_or_argument_ty)?;
                Some(quote!(#ty))
            }
            FlagOrValue::Value { value: ty, .. } => Some(quote!(#ty)),
        };

        return match ty {
            Some(ty) => {
                let create_single = match color {
                    Color::Async => parse_quote! {
                        cx.try_just_create_singletons_by_type_async::<#ty>().await;
                    },
                    Color::Sync => parse_quote! {
                        cx.try_just_create_singletons_by_type::<#ty>();
                    },
                };

                let get_singleton = parse_quote! {
                    let #ident = cx.get_singletons_by_type();
                };

                Ok(ResolveOne {
                    stmt: ResolveOneValue::Ref {
                        create_single,
                        get_singleton,
                    },
                    variable: ident,
                })
            }
            None => {
                let resolve = match color {
                    Color::Async => parse_quote! {
                        let #ident = cx.resolve_by_type_async().await;
                    },
                    Color::Sync => parse_quote! {
                        let #ident = cx.resolve_by_type();
                    },
                };

                Ok(ResolveOne {
                    stmt: ResolveOneValue::Owned { resolve },
                    variable: ident,
                })
            }
        };
    }

    // A map of the instances of a type, keyed by the name each one was registered
    // under.
    //
    // The name comes from the context, which is the only place that knows it: an
    // instance is not asked for its own name, because a type that is registered
    // under one has no reason to carry it — requiring `singleton_name()` would
    // mean every injectable type had to implement a trait it never uses.
    //
    // Which map it becomes is not the macro's business either: the declared type
    // of the field or the argument decides, so all that is generated is the
    // collection of `(String, T)` pairs, and `HashMap` and `BTreeMap` both take
    // it.
    if map {
        let ty = match ref_ {
            FlagOrValue::None => None,
            FlagOrValue::Flag { .. } => {
                // The declared type carries the reference the instance is lent
                // as, which the resolver is not asked for: it hands out
                // `(String, &T)` itself.
                let ty = extract_hashmap_type(field_or_argument_ty)?;
                let ty = match ty {
                    Type::Reference(reference) => *reference.elem,
                    other => other,
                };

                Some(quote!(#ty))
            }
            FlagOrValue::Value { value: ty, .. } => Some(quote!(#ty)),
        };

        return match ty {
            Some(ty) => {
                let create_single = match color {
                    Color::Async => parse_quote! {
                        cx.try_just_create_singletons_by_type_async::<#ty>().await;
                    },
                    Color::Sync => parse_quote! {
                        cx.try_just_create_singletons_by_type::<#ty>();
                    },
                };

                let get_singleton = parse_quote! {
                    let #ident: #field_or_argument_ty =
                        cx.get_singletons_by_type_with_names::<#ty>().into_iter().collect();
                };

                Ok(ResolveOne {
                    stmt: ResolveOneValue::Ref {
                        create_single,
                        get_singleton,
                    },
                    variable: ident,
                })
            }
            None => {
                let ty = extract_hashmap_type(field_or_argument_ty)?;
                let resolve = match color {
                    Color::Async => parse_quote! {
                        let #ident: #field_or_argument_ty =
                            cx.resolve_by_type_with_names_async::<#ty>().await.into_iter().collect();
                    },
                    Color::Sync => parse_quote! {
                        let #ident: #field_or_argument_ty =
                            cx.resolve_by_type_with_names::<#ty>().into_iter().collect();
                    },
                };

                Ok(ResolveOne {
                    stmt: ResolveOneValue::Owned { resolve },
                    variable: ident,
                })
            }
        };
    }

    if let Some(is_mut_ref) = inferred_ref {
        let instance_ty = strip_references(field_or_argument_ty);

        // The instance is created in the context first, so that the item can
        // borrow it: a `&T` borrows it immutably and a `&mut T` mutably, which
        // is what makes an item able to modify a singleton.
        let create_single = match color {
            Color::Async => parse_quote! {
                cx.just_create_singleton_with_name_async::<#instance_ty>(#name).await;
            },
            Color::Sync => parse_quote! {
                cx.just_create_singleton_with_name::<#instance_ty>(#name);
            },
        };

        let get_singleton = match is_mut_ref {
            false => parse_quote! {
                let #ident = cx.get_singleton_with_name(#name);
            },
            true => parse_quote! {
                let #ident = cx.get_singleton_mut_with_name(#name);
            },
        };

        return Ok(ResolveOne {
            stmt: ResolveOneValue::Ref {
                create_single,
                get_singleton,
            },
            variable: ident,
        });
    }

    let ty = match ref_ {
        FlagOrValue::None => None,
        FlagOrValue::Flag { .. } => {
            let ty = extract_ref_type(field_or_argument_ty)?;
            Some(quote!(#ty))
        }
        FlagOrValue::Value { value: ty, .. } => Some(quote!(#ty)),
    };

    match ty {
        Some(ty) => {
            let create_single = match color {
                Color::Async => parse_quote! {
                    cx.just_create_singleton_with_name_async::<#ty>(#name).await;
                },
                Color::Sync => parse_quote! {
                    cx.just_create_singleton_with_name::<#ty>(#name);
                },
            };

            let get_singleton = parse_quote! {
                let #ident = cx.get_singleton_with_name(#name);
            };

            Ok(ResolveOne {
                stmt: ResolveOneValue::Ref {
                    create_single,
                    get_singleton,
                },
                variable: ident,
            })
        }
        None => {
            // `#[value("key")]` reads the value the application's environment
            // holds, instead of resolving a singleton: the instance is configured,
            // not injected.
            //
            // It is read here, in the statement both colours share, because there
            // is nothing to await — the environment is in place before any
            // provider is built. Handling it only in the synchronous arm would
            // make an asynchronous constructor silently resolve a singleton named
            // after the key instead of reading it.
            if is_value {
                // The framework registers its environment under this name. The
                // constant for it lives in `next-web-context`, which the generated
                // code cannot name from every crate that uses these macros, so the
                // name is spelled out.
                let environment_singleton_name =
                    syn::LitStr::new("applicationEnvironment", Span::call_site());

                // An `Option<T>` field means the property may be absent; any other
                // type means the application cannot start without it.
                // What a property that is not configured at all becomes. The
                // attribute may name it, and otherwise the type's own default is
                // used, so a missing property is never an error.
                let fallback = match &value_default {
                    Some(expr) => quote!(#expr),
                    None => quote!(::core::default::Default::default()),
                };

                // Reading the inner type is what tells an `Option<T>` field from a
                // plain one. Neither of the other two helpers can do it:
                // `extract_path_type_with_owned_type` only unwraps the two
                // arguments of a map and reports an error for a single one, and
                // `extract_option_type` strips a reference from what it found,
                // which an `Option<T>` field does not have.
                let read =
                    match crate::util::extract_type::extract_option_inner_type(field_or_argument_ty)
                    {
                        Some(inner) => parse_quote! {
                        let #ident: #field_or_argument_ty = match cx
                            .get_singleton_with_name::<::std::sync::Arc<dyn ::next_web::core::env::ConfigurableEnvironment>>(
                                #environment_singleton_name,
                            )
                            .get_property(#key)
                        {
                            ::core::option::Option::Some(__value) => ::core::option::Option::Some(
                                match __value.parse::<#inner>() {
                                    ::core::result::Result::Ok(__parsed) => __parsed,
                                    ::core::result::Result::Err(_) => ::core::panic!(
                                        "the property `{}` is not a valid value: {:?}",
                                        #key,
                                        __value,
                                    ),
                                },
                            ),
                            ::core::option::Option::None => #fallback,
                        };
                    },
                    None => parse_quote! {
                        let #ident: #field_or_argument_ty = match cx
                            .get_singleton_with_name::<::std::sync::Arc<dyn ::next_web::core::env::ConfigurableEnvironment>>(
                                #environment_singleton_name,
                            )
                            .get_property(#key)
                        {
                            ::core::option::Option::Some(__value) => match __value.parse() {
                                ::core::result::Result::Ok(__parsed) => __parsed,
                                ::core::result::Result::Err(_) => ::core::panic!(
                                    "the property `{}` is not a valid value: {:?}",
                                    #key,
                                    __value,
                                ),
                            },
                            ::core::option::Option::None => #fallback,
                        };
                    },
                };

                return Ok(ResolveOne {
                    stmt: ResolveOneValue::Owned { resolve: read },
                    variable: ident,
                });
            }

            let resolve = match color {
                Color::Async => parse_quote! {
                    let #ident = cx.resolve_with_name_async(#name).await;
                },
                Color::Sync => parse_quote! {
                    let #ident = cx.resolve_with_name(#name);
                },
            };

            Ok(ResolveOne {
                stmt: ResolveOneValue::Owned { resolve },
                variable: ident,
            })
        }
    }
}

pub(crate) struct ArgumentResolveStmts {
    pub(crate) ref_mut_cx_stmts: Vec<Stmt>,
    pub(crate) ref_cx_stmts: Vec<Stmt>,
    pub(crate) args: Vec<Ident>,
}

/// Adds the statements a resolved field or argument needs to the two sets of
/// statements a constructor runs before it builds its instance.
///
/// # Arguments
///
/// * `stmt` - The statements produced for one field or argument.
/// * `ref_mut_cx_stmts` - The statements that need the context mutably.
/// * `ref_cx_stmts` - The statements that borrow the instances the first set
///   stored in the context.
pub(crate) fn push_resolve_stmts(
    stmt: ResolveOneValue,
    ref_mut_cx_stmts: &mut Vec<Stmt>,
    ref_cx_stmts: &mut Vec<Stmt>,
) {
    match stmt {
        ResolveOneValue::Owned { resolve } => ref_mut_cx_stmts.push(resolve),
        ResolveOneValue::Ref {
            create_single,
            get_singleton,
        } => {
            ref_mut_cx_stmts.push(create_single);
            ref_cx_stmts.push(get_singleton);
        }
    }
}

/// The mutability of a reference the context hands out, and where it was
/// written down.
pub(crate) type BorrowedReference = (Span, bool);

/// Checks that the references the context hands out to an item can exist at the
/// same time.
///
/// The context is borrowed while the item runs, so it can lend any number of
/// immutable references, or a single mutable reference, but not a mutable
/// reference together with another one: taking `&mut T` from the context and
/// then borrowing the context again does not compile.
///
/// # Arguments
///
/// * `references` - The references of the fields or arguments of the item,
///   paired with their mutability.
pub(crate) fn check_reference_borrows(references: &[BorrowedReference]) -> syn::Result<()> {
    let mut mutables = references
        .iter()
        .filter(|(_, is_mut)| *is_mut)
        .map(|(span, _)| *span);

    if mutables.next().is_none() {
        return Ok(());
    }

    if let Some(span) = mutables.next() {
        return Err(syn::Error::new(
            span,
            "a mutable reference is borrowed from the context while the item runs, \
            so only one `&mut` field or argument is supported",
        ));
    }

    if let Some((span, _)) = references.iter().find(|(_, is_mut)| !is_mut) {
        return Err(syn::Error::new(
            *span,
            "a mutable reference is borrowed from the context while the item runs, \
            so a `&mut` field or argument cannot be combined with another reference, \
            please make one of them an owned field or argument",
        ));
    }

    Ok(())
}

pub(crate) fn generate_argument_resolve_methods(
    inputs: &mut Punctuated<FnArg, Token![,]>,
    color: Color,
) -> syn::Result<ArgumentResolveStmts> {
    let capacity = inputs.len();

    let mut ref_mut_cx_stmts = Vec::with_capacity(capacity);
    let mut ref_cx_stmts = Vec::with_capacity(capacity);
    let mut args = Vec::with_capacity(capacity);
    let mut references = Vec::new();

    for (index, input) in inputs.iter_mut().enumerate() {
        match input {
            FnArg::Receiver(r) => {
                return Err(syn::Error::new(r.span(), "not support `self` receiver"));
            }
            FnArg::Typed(PatType { attrs, pat, ty, .. }) => {
                if let Type::Reference(TypeReference { mutability, .. }) = &**ty {
                    references.push((ty.span(), mutability.is_some()));
                }

                // The name of the instance an argument takes is the name of the
                // argument, so that a provider function of the same name needs
                // no `name` argument.
                let argument_name = match &**pat {
                    Pat::Ident(PatIdent { ident, .. }) => Some(ident.clone()),
                    _ => None,
                };

                let ResolveOne { stmt, variable } =
                    generate_only_one_field_or_argument_resolve_stmt(
                        argument_name,
                        attrs,
                        color,
                        index,
                        ty,
                    )?;

                push_resolve_stmts(stmt, &mut ref_mut_cx_stmts, &mut ref_cx_stmts);

                args.push(variable);
            }
        }
    }

    check_reference_borrows(&references)?;

    Ok(ArgumentResolveStmts {
        ref_mut_cx_stmts: with_context_imports(ref_mut_cx_stmts, color),
        ref_cx_stmts,
        args,
    })
}

#[cfg(feature = "auto-register")]
pub(crate) enum ItemKind {
    Struct,
    Enum,
    Function,

    // impl block
    StructOrEnum,
}

#[cfg(feature = "auto-register")]
impl ItemKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            ItemKind::Struct => "struct",
            ItemKind::Enum => "enum",
            ItemKind::Function => "function",
            ItemKind::StructOrEnum => "struct or enum",
        }
    }
}

#[cfg(feature = "auto-register")]
pub(crate) fn check_generics_when_enable_auto_register(
    auto_register: bool,
    generics: &syn::Generics,
    item_kind: ItemKind,
    scope: Scope,
) -> syn::Result<()> {
    if auto_register && !generics.params.is_empty() {
        return Err(syn::Error::new(
            generics.span(),
            format!(
                "not support auto register generics {}, \
                please remove generics, or use `#[{:?}(auto_register = false)]` to disable auto register",
                item_kind.as_str(),
                scope
            ),
        ));
    }

    Ok(())
}

pub(crate) struct FieldResolveStmts {
    pub(crate) ref_mut_cx_stmts: Vec<Stmt>,
    pub(crate) ref_cx_stmts: Vec<Stmt>,
    pub(crate) fields: ResolvedFields,
}

pub(crate) enum ResolvedFields {
    Unit,
    Named {
        field_names: Vec<Ident>,
        field_values: Vec<Ident>,
    },
    Unnamed(Vec<Ident>),
}

pub(crate) fn generate_field_resolve_stmts(
    fields: &mut Fields,
    color: Color,
) -> syn::Result<FieldResolveStmts> {
    match fields {
        Fields::Unit => Ok(FieldResolveStmts {
            ref_mut_cx_stmts: Vec::new(),
            ref_cx_stmts: Vec::new(),
            fields: ResolvedFields::Unit,
        }),
        Fields::Named(FieldsNamed { named, .. }) => {
            let capacity = named.len();

            let mut ref_mut_cx_stmts = Vec::with_capacity(capacity);
            let mut ref_cx_stmts = Vec::with_capacity(capacity);
            let mut field_values = Vec::with_capacity(capacity);

            let mut field_names = Vec::with_capacity(capacity);
            let mut references = Vec::new();

            for (
                index,
                Field {
                    attrs,
                    ident: field_name,
                    ty,
                    ..
                },
            ) in named.into_iter().enumerate()
            {
                if let Type::Reference(TypeReference { mutability, .. }) = &*ty {
                    let is_mut_ref = mutability.is_some();
                    references.push((ty.span(), is_mut_ref));
                }

                let ResolveOne {
                    stmt,
                    variable: field_value,
                } = generate_only_one_field_or_argument_resolve_stmt(
                    field_name.clone(),
                    attrs,
                    color,
                    index,
                    ty,
                )?;

                push_resolve_stmts(stmt, &mut ref_mut_cx_stmts, &mut ref_cx_stmts);

                field_values.push(field_value);
                field_names.push(field_name.clone().unwrap());
            }

            check_reference_borrows(&references)?;

            Ok(FieldResolveStmts {
                ref_mut_cx_stmts: with_context_imports(ref_mut_cx_stmts, color),
                ref_cx_stmts,
                fields: ResolvedFields::Named {
                    field_names,
                    field_values,
                },
            })
        }
        Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
            let capacity = unnamed.len();

            let mut ref_mut_cx_stmts = Vec::with_capacity(capacity);
            let mut ref_cx_stmts = Vec::with_capacity(capacity);
            let mut field_values = Vec::with_capacity(capacity);
            let mut references = Vec::new();

            for (index, Field { attrs, ty, .. }) in unnamed.into_iter().enumerate() {
                if let Type::Reference(TypeReference { mutability, .. }) = &*ty {
                    let is_mut_ref = mutability.is_some();
                    references.push((ty.span(), is_mut_ref));
                }

                let ResolveOne {
                    stmt,
                    variable: field_value,
                } = generate_only_one_field_or_argument_resolve_stmt(
                    None, attrs, color, index, ty,
                )?;

                push_resolve_stmts(stmt, &mut ref_mut_cx_stmts, &mut ref_cx_stmts);

                field_values.push(field_value);
            }

            check_reference_borrows(&references)?;

            Ok(FieldResolveStmts {
                ref_mut_cx_stmts: with_context_imports(ref_mut_cx_stmts, color),
                ref_cx_stmts,
                fields: ResolvedFields::Unnamed(field_values),
            })
        }
    }
}

fn extract_vec_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty {
        let last_segment = type_path.path.segments.last()?;

        // 检查是否是 Option
        if last_segment.ident == "Option" {
            // 获取泛型参数
            if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                // 获取第一个泛型参数
                if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                    return Some(inner_ty);
                }
            }
        }
    }
    None
}
