//! Tests of the code the dependency injection macros generate.
//!
//! The macros are exercised through the functions that produce their code, so
//! that the generated code can be asserted on without an application context.

use from_attr::FromAttr;
use next_web_context::{Color, Scope};
use quote::quote;
use syn::{parse_quote, ItemFn, Signature};

use crate::di::{
    commons::{self, ArgumentResolveStmts, FieldResolveStmts},
    item_fn_gen,
    struct_or_function_attr::StructOrFunctionAttr,
};

/// Returns code with its whitespace normalized, so that it can be asserted on
/// without depending on the formatting of [`quote`].
fn code_of(tokens: impl ToString) -> String {
    tokens
        .to_string()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Returns the code of the two sets of statements a constructor runs before it
/// calls the item it provides, and of the arguments it is called with.
///
/// The first set needs the context mutably, the second one borrows the
/// instances the first set stored in the context.
fn resolve_of(signature: &str, color: Color) -> (String, String, String) {
    let Signature { mut inputs, .. } =
        syn::parse_str::<Signature>(signature).expect("the signature is parsable");

    let ArgumentResolveStmts {
        ref_mut_cx_stmts,
        ref_cx_stmts,
        args,
    } = commons::generate_argument_resolve_methods(&mut inputs, color)
        .expect("the arguments of the signature are resolvable");

    (
        code_of(quote!(#(#ref_mut_cx_stmts)*)),
        code_of(quote!(#(#ref_cx_stmts)*)),
        code_of(quote!(#(#args)*)),
    )
}

/// Returns the message the references of a signature are rejected with.
fn rejection_of(signature: &str) -> String {
    let Signature { mut inputs, .. } =
        syn::parse_str::<Signature>(signature).expect("the signature is parsable");

    match commons::generate_argument_resolve_methods(&mut inputs, Color::Sync) {
        Ok(_) => panic!("the references of the signature should not be accepted"),
        Err(error) => error.to_string(),
    }
}

/// Returns the code a function provider is generated from.
fn provider_of(attr: &str, item_fn: ItemFn, scope: Scope) -> String {
    let attr = syn::parse_str::<proc_macro2::TokenStream>(attr).expect("the attribute is parsable");
    let attr = StructOrFunctionAttr::from_tokens(attr).expect("the attribute is valid");

    code_of(item_fn_gen::generate(attr, item_fn, scope).expect("the function is valid"))
}

/// Returns the code of the two sets of statements a constructor of a struct
/// provider runs before it builds its instance.
fn resolve_fields_of(item_struct: &str) -> (String, String) {
    let syn::ItemStruct { mut fields, .. } =
        syn::parse_str::<syn::ItemStruct>(item_struct).expect("the struct is parsable");

    let FieldResolveStmts {
        ref_mut_cx_stmts,
        ref_cx_stmts,
        ..
    } = commons::generate_field_resolve_stmts(&mut fields, Color::Sync)
        .expect("the fields of the tuple struct are resolvable");

    (
        code_of(quote!(#(#ref_mut_cx_stmts)*)),
        code_of(quote!(#(#ref_cx_stmts)*)),
    )
}

#[test]
fn a_function_provider_is_named_after_the_function() {
    let provider = provider_of(
        "",
        parse_quote! {
            fn test_name() -> String {
                String::new()
            }
        },
        Scope::Singleton,
    );

    assert!(provider.contains(r#". name ("testName")"#), "{provider}");
}

#[test]
fn a_named_function_provider_keeps_the_name_of_the_attribute() {
    let provider = provider_of(
        r#"name = "hello""#,
        parse_quote! {
            fn test_name() -> String {
                String::new()
            }
        },
        Scope::Singleton,
    );

    assert!(provider.contains(r#". name ("hello")"#), "{provider}");
    assert!(!provider.contains(r#"("testName")"#), "{provider}");
}

#[test]
fn the_type_of_a_function_provider_is_not_a_unit_struct() {
    let provider = provider_of(
        "",
        parse_quote! {
            fn state() -> String {
                String::new()
            }
        },
        Scope::Singleton,
    );

    // A unit struct is read as a pattern, so a field or an argument that is
    // named like the function could not be a binding of its own.
    assert!(!provider.contains("struct state ;"), "{provider}");
    assert!(provider.contains("_mark : ()"), "{provider}");
}

#[test]
fn a_mutable_reference_argument_is_taken_from_the_context() {
    let (context, instances, args) = resolve_of(
        r#"fn test_name(
            #[autowired(name = "testName")] name: &mut String,
            greeting: String,
        ) -> String"#,
        Color::Sync,
    );

    // The instance is created in the context, and the owned argument is
    // resolved with it, before the context is borrowed.
    assert!(
        context.contains(r#"just_create_singleton_with_name :: < String > ("testName")"#),
        "{context}"
    );
    assert!(
        context.contains(r#"let owned_1 = cx . resolve_with_name ("greeting") ;"#),
        "{context}"
    );

    // The mutable reference is borrowed from the context, so the item modifies
    // the instance the context keeps.
    assert!(
        instances.contains(r#"let ref_0 = cx . get_singleton_mut_with_name ("testName") ;"#),
        "{instances}"
    );

    assert_eq!(args, "ref_0 owned_1");
}

#[test]
fn immutable_reference_arguments_are_taken_from_the_context() {
    let (context, instances, args) = resolve_of(
        r#"fn test_name(
            #[autowired(name = "testName")] name: &String,
            #[autowired(name = "array")] array: &Vec<String>,
            greeting: String,
        ) -> String"#,
        Color::Sync,
    );

    assert!(
        context.contains(r#"just_create_singleton_with_name :: < String > ("testName")"#),
        "{context}"
    );
    assert!(
        context.contains(r#"just_create_singleton_with_name :: < Vec < String > > ("array")"#),
        "{context}"
    );
    assert!(
        context.contains(r#"let owned_2 = cx . resolve_with_name ("greeting") ;"#),
        "{context}"
    );

    // Any number of immutable references can be borrowed at the same time.
    assert!(
        instances.contains(r#"let ref_0 = cx . get_singleton_with_name ("testName") ;"#),
        "{instances}"
    );
    assert!(
        instances.contains(r#"let ref_1 = cx . get_singleton_with_name ("array") ;"#),
        "{instances}"
    );

    assert_eq!(args, "ref_0 ref_1 owned_2");
}

#[test]
fn an_owned_argument_is_handed_over_as_it_is() {
    let (context, instances, args) = resolve_of(
        r#"fn test_name(#[autowired(name = "testName")] name: String) -> String"#,
        Color::Sync,
    );

    assert!(
        context.contains(r#"let owned_0 = cx . resolve_with_name ("testName") ;"#),
        "{context}"
    );
    assert!(instances.is_empty(), "{instances}");
    assert_eq!(args, "owned_0");
}

#[test]
fn an_argument_without_a_name_is_taken_from_the_name_of_the_argument() {
    let (context, instances, args) = resolve_of(
        r#"fn test_name(my_service: String, my_array: &Vec<String>) -> String"#,
        Color::Sync,
    );

    // The name of a provider function of the same name is the one of the
    // argument, in camel case.
    assert!(
        context.contains(r#"let owned_0 = cx . resolve_with_name ("myService") ;"#),
        "{context}"
    );
    assert!(
        context.contains(r#"just_create_singleton_with_name :: < Vec < String > > ("myArray")"#),
        "{context}"
    );
    assert!(
        instances.contains(r#"let ref_1 = cx . get_singleton_with_name ("myArray") ;"#),
        "{instances}"
    );

    assert_eq!(args, "owned_0 ref_1");
}

#[test]
fn the_name_of_the_attribute_wins_over_the_name_of_the_argument() {
    let (context, _, _) = resolve_of(
        r#"fn test_name(#[autowired(name = "explicit")] my_service: String) -> String"#,
        Color::Sync,
    );

    assert!(
        context.contains(r#"let owned_0 = cx . resolve_with_name ("explicit") ;"#),
        "{context}"
    );
    assert!(!context.contains("myService"), "{context}");
}

#[test]
fn a_leading_underscore_is_not_part_of_the_name_of_an_argument() {
    let (context, _, _) = resolve_of(
        r#"fn test_name(_my_service: String) -> String"#,
        Color::Sync,
    );

    assert!(
        context.contains(r#"let owned_0 = cx . resolve_with_name ("myService") ;"#),
        "{context}"
    );
}

#[test]
fn a_field_without_a_name_is_taken_from_the_lower_cased_name_of_its_type() {
    let (context, instances) = resolve_fields_of("pub struct Fields<'a>(&'a MyService);");

    assert!(
        context.contains(r#"just_create_singleton_with_name :: < MyService > ("myService")"#),
        "{context}"
    );
    assert!(
        instances.contains(r#"let ref_0 = cx . get_singleton_with_name ("myService") ;"#),
        "{instances}"
    );
}

#[test]
fn a_reference_argument_of_an_async_function_is_created_async() {
    let (context, instances, _) = resolve_of(
        r#"fn test_name(#[autowired(name = "testName")] name: &mut String) -> String"#,
        Color::Async,
    );

    assert!(
        context
            .contains(r#"just_create_singleton_with_name_async :: < String > ("testName") . await"#),
        "{context}"
    );
    assert!(
        instances.contains(r#"let ref_0 = cx . get_singleton_mut_with_name ("testName") ;"#),
        "{instances}"
    );
}

#[test]
fn an_explicit_ref_argument_is_taken_from_the_context() {
    let (context, instances, args) = resolve_of(
        r#"fn test_name(#[autowired(ref, name = "testName")] name: &String) -> String"#,
        Color::Sync,
    );

    assert!(
        context.contains(r#"just_create_singleton_with_name :: < String > ("testName")"#),
        "{context}"
    );
    assert!(
        instances.contains(r#"let ref_0 = cx . get_singleton_with_name ("testName")"#),
        "{instances}"
    );
    assert_eq!(args, "ref_0");
}

#[test]
fn a_mutable_reference_next_to_another_reference_is_rejected() {
    let rejection = rejection_of(
        r#"fn test_name(
            #[autowired(name = "testName")] name: &mut String,
            #[autowired(name = "array")] array: &Vec<String>,
        ) -> String"#,
    );

    assert!(
        rejection.contains("cannot be combined with another reference"),
        "{rejection}"
    );
}

#[test]
fn only_one_mutable_reference_is_supported() {
    let rejection = rejection_of(
        r#"fn test_name(
            #[autowired(name = "first")] first: &mut String,
            #[autowired(name = "second")] second: &mut String,
        ) -> String"#,
    );

    assert!(
        rejection.contains("only one `&mut` field or argument is supported"),
        "{rejection}"
    );
}

#[test]
fn a_missing_instance_of_an_optional_reference_is_created_without_failing() {
    let (context, instances, args) = resolve_of(
        r#"fn test_name(
            #[autowired(option, ref = String, name = "testName")] name: Option<&String>,
        ) -> String"#,
        Color::Sync,
    );

    // The instance is created when it is missing, and the item still resolves
    // when the context has no provider for it.
    assert!(
        context.contains(
            r#"cx . try_just_create_singleton_with_name :: < String > ("testName") ;"#
        ),
        "{context}"
    );
    assert!(
        instances.contains(r#"let ref_0 = cx . get_singleton_option_with_name ("testName") ;"#),
        "{instances}"
    );
    assert_eq!(args, "ref_0");
}

#[test]
fn every_instance_of_a_type_is_created_before_they_are_borrowed() {
    let (context, instances, args) = resolve_of(
        r#"fn test_name(#[autowired(vec, ref = String)] names: Vec<&String>) -> String"#,
        Color::Sync,
    );

    assert!(
        context.contains(r#"cx . try_just_create_singletons_by_type :: < String > () ;"#),
        "{context}"
    );
    assert!(
        instances.contains(r#"let ref_0 = cx . get_singletons_by_type () ;"#),
        "{instances}"
    );
    assert_eq!(args, "ref_0");
}

#[test]
fn every_instance_of_a_type_is_keyed_by_its_singleton_name() {
    let (context, instances, args) = resolve_of(
        r#"fn test_name(
            #[autowired(map, ref = String)] names: HashMap<String, &String>,
        ) -> String"#,
        Color::Sync,
    );

    assert!(
        context.contains(r#"cx . try_just_create_singletons_by_type :: < String > () ;"#),
        "{context}"
    );
    // The instances are keyed by their singleton name, like the owned variant
    // of the attribute.
    assert!(
        instances.contains(r#"instance . singleton_name ()"#),
        "{instances}"
    );
    assert_eq!(args, "ref_0");
}
