//! Tests of the code the `#[configuration]` and `#[auto_configuration]` macros
//! generate.
//!
//! The macro is exercised through the function that produces its code, so that
//! the generated code can be asserted on without an application. The assertions
//! use the testing support of `zyn`, which prints the code the macro generated
//! when an assertion fails.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::web::auto_configuration::{generate_auto_configuration, generate_configuration};

/// Returns the code of the configuration class of an annotated implementation
/// block.
///
/// # Arguments
///
/// * `attrs` - The attributes of the macro.
/// * `item_impl` - The annotated implementation block.
fn code_of(attrs: TokenStream2, item_impl: TokenStream2) -> TokenStream2 {
    let item_impl = syn::parse2(item_impl).expect("the implementation block is parsable");

    generate_auto_configuration(attrs, item_impl).expect("the configuration is generated")
}

/// Returns the implementation block the tests generate a configuration class
/// for, which declares one provider.
fn configuration() -> TokenStream2 {
    quote! {
        impl MyConfiguration {
            #[provider(name = "myName")]
            fn my_name() -> String {
                String::from("name")
            }
        }
    }
}

#[test]
fn names_the_configuration_after_the_annotated_type() {
    let code = code_of(quote! {}, configuration());

    zyn::assert_tokens_contain!(
        code,
        r#"fn auto_configuration_name (& self) -> & 'static str { "MyConfiguration" }"#
    );
}

#[test]
fn declares_the_place_of_the_configuration_relative_to_other_configurations() {
    let code = code_of(
        quote! {
            before = ["RedisAutoConfiguration", "AnotherConfiguration"],
            after = ["MessageSourceAutoConfiguration"]
        },
        configuration(),
    );

    zyn::assert_tokens_contain!(
        code,
        r#"fn auto_configure_before (& self) -> & 'static [& 'static str] { & ["RedisAutoConfiguration" , "AnotherConfiguration"] }"#
    );
    zyn::assert_tokens_contain!(
        code,
        r#"fn auto_configure_after (& self) -> & 'static [& 'static str] { & ["MessageSourceAutoConfiguration"] }"#
    );
}

#[test]
fn does_not_declare_a_place_that_the_configuration_does_not_declare() {
    let code = code_of(quote! {}, configuration());

    let code = code.to_string();

    assert!(!code.contains("auto_configure_before"), "{code}");
    assert!(!code.contains("auto_configure_after"), "{code}");
}

#[test]
fn rejects_a_name_that_is_empty() {
    let item_impl = syn::parse2(configuration()).expect("the implementation block is parsable");

    let error = generate_auto_configuration(quote! { before = [""] }, item_impl)
        .err()
        .expect("an empty name is rejected");

    assert!(
        error.to_string().contains("`before`"),
        "the error names the attribute: {error}"
    );
}

/// Returns the implementation block of a configuration whose single provider
/// declares the given signature, which the tests of the arguments of a provider
/// expand.
///
/// # Arguments
///
/// * `signature` - The signature of the provider.
fn provider_with(signature: &str) -> TokenStream2 {
    format!("impl MyConfiguration {{ #[provider] {signature} }}")
        .parse()
        .expect("the implementation block is parsable")
}

#[test]
fn borrows_the_instances_the_reference_arguments_name_from_the_context() {
    let code = code_of(
        quote! {},
        provider_with(
            "fn my_name(first: &FirstService, second: &SecondService) -> String { \
             String::new() }",
        ),
    );

    // The instances are created first, because the context can only lend an
    // instance it holds, and the provider borrows them afterwards.
    zyn::assert_tokens_contain!(
        code,
        r#"ctx . try_just_create_singleton_with_name :: < FirstService > ("first") ;"#
    );
    zyn::assert_tokens_contain!(
        code,
        r#"ctx . try_just_create_singleton_with_name :: < SecondService > ("second") ;"#
    );
    zyn::assert_tokens_contain!(
        code,
        r#"let ref0 = ctx . get_singleton_with_name :: < FirstService > ("first") ;"#
    );
    zyn::assert_tokens_contain!(
        code,
        r#"let ref1 = ctx . get_singleton_with_name :: < SecondService > ("second") ;"#
    );
    zyn::assert_tokens_contain!(code, "let instance = my_name (ref0 , ref1) ;");

    // A reference the context lends is taken after every statement that needs
    // the context mutably, because the context cannot be borrowed twice.
    let code = code.to_string();
    let created = code
        .find(r#"try_just_create_singleton_with_name :: < SecondService >"#)
        .expect("the second instance is created");
    let borrowed = code
        .find(r#"get_singleton_with_name :: < FirstService >"#)
        .expect("the first instance is borrowed");

    assert!(
        created < borrowed,
        "the instances are created before they are borrowed: {code}"
    );
}

#[test]
fn borrows_the_instance_a_mutable_reference_argument_names() {
    let code = code_of(
        quote! {},
        provider_with(
            "fn my_name(services: &mut ServiceRegistry, name: String) -> String { \
             String::new() }",
        ),
    );

    zyn::assert_tokens_contain!(
        code,
        r#"ctx . try_just_create_singleton_with_name :: < ServiceRegistry > ("services") ;"#
    );
    zyn::assert_tokens_contain!(
        code,
        r#"let ref0 = ctx . get_singleton_mut_with_name :: < ServiceRegistry > ("services") ;"#
    );
    // The owned argument is resolved before the context lends its instance, so
    // that a provider can take an instance mutably and a value as well.
    zyn::assert_tokens_contain!(
        code,
        r#"let owned1 = ctx . resolve_with_name :: < String > ("name") ;"#
    );
    zyn::assert_tokens_contain!(code, "let instance = my_name (ref0 , owned1) ;");
}

#[test]
fn names_the_instance_a_reference_argument_borrows() {
    let code = code_of(
        quote! {},
        quote! {
            impl MyConfiguration {
                #[provider]
                fn my_name(#[autowired(name = "otherService")] service: &FirstService) -> String {
                    String::new()
                }
            }
        },
    );

    zyn::assert_tokens_contain!(
        code,
        r#"ctx . try_just_create_singleton_with_name :: < FirstService > ("otherService") ;"#
    );
    zyn::assert_tokens_contain!(
        code,
        r#"let ref0 = ctx . get_singleton_with_name :: < FirstService > ("otherService") ;"#
    );
}

#[test]
fn rejects_a_second_mutable_reference() {
    let item_impl = syn::parse2(provider_with(
        "fn my_name(first: &mut FirstService, second: &mut SecondService) -> String { \
         String::new() }",
    ))
    .expect("the implementation block is parsable");

    let error = generate_auto_configuration(TokenStream2::new(), item_impl)
        .err()
        .expect("two mutable references are rejected");

    assert!(
        error.to_string().contains("only one `&mut`"),
        "the error explains the rule: {error}"
    );
}

#[test]
fn rejects_a_mutable_reference_next_to_another_reference() {
    let item_impl = syn::parse2(provider_with(
        "fn my_name(first: &mut FirstService, second: &SecondService) -> String { \
         String::new() }",
    ))
    .expect("the implementation block is parsable");

    let error = generate_auto_configuration(TokenStream2::new(), item_impl)
        .err()
        .expect("a mutable reference next to another reference is rejected");

    assert!(
        error.to_string().contains("cannot be combined"),
        "the error explains the rule: {error}"
    );
}

#[test]
fn rejects_a_default_on_a_reference() {
    for signature in [
        "fn my_name(#[autowired(default)] first: &FirstService) -> String { String::new() }",
        "fn my_name(#[autowired(default)] first: &mut FirstService) -> String { String::new() }",
    ] {
        let item_impl =
            syn::parse2(provider_with(signature)).expect("the implementation block is parsable");

        let error = generate_auto_configuration(TokenStream2::new(), item_impl)
            .err()
            .unwrap_or_else(|| panic!("a default on `{signature}` is rejected"));

        assert!(
            error.to_string().contains("cannot fall back to a default"),
            "the error explains the rule: {error}"
        );
    }
}

/// Returns the code of the configuration class of a `#[configuration]` block.
///
/// # Arguments
///
/// * `attrs` - The attributes of the macro.
/// * `item_impl` - The annotated implementation block.
fn configuration_code_of(attrs: TokenStream2, item_impl: TokenStream2) -> TokenStream2 {
    let item_impl = syn::parse2(item_impl).expect("the implementation block is parsable");

    generate_configuration(attrs, item_impl).expect("the configuration is generated")
}

#[test]
fn names_the_configuration_class_after_the_annotated_type() {
    let code = configuration_code_of(quote! {}, configuration());

    zyn::assert_tokens_contain!(
        code,
        r#"fn configuration_name (& self) -> & 'static str { "MyConfiguration" }"#
    );
}

#[test]
fn declares_the_place_of_the_configuration_class() {
    let code = configuration_code_of(
        quote! {
            before = ["RedisConfiguration"],
            after = ["MessageSourceConfiguration"]
        },
        configuration(),
    );

    zyn::assert_tokens_contain!(
        code,
        r#"fn configure_before (& self) -> & 'static [& 'static str] { & ["RedisConfiguration"] }"#
    );
    zyn::assert_tokens_contain!(
        code,
        r#"fn configure_after (& self) -> & 'static [& 'static str] { & ["MessageSourceConfiguration"] }"#
    );
}

#[test]
fn runs_an_initializer_without_registering_a_value() {
    let code = configuration_code_of(
        quote! {},
        quote! {
            impl MyConfiguration {
                #[init]
                fn register(registry: &mut ServiceRegistry, name: String) {
                    registry.register(&name);
                }
            }
        },
    );

    zyn::assert_tokens_contain!(
        code,
        r#"ctx . try_just_create_singleton_with_name :: < ServiceRegistry > ("registry") ;"#
    );
    zyn::assert_tokens_contain!(
        code,
        r#"let ref0 = ctx . get_singleton_mut_with_name :: < ServiceRegistry > ("registry") ;"#
    );
    zyn::assert_tokens_contain!(
        code,
        r#"let owned1 = ctx . resolve_with_name :: < String > ("name") ;"#
    );
    zyn::assert_tokens_contain!(code, "register (ref0 , owned1) ;");

    // An initializer does not register a value, which is what keeps it apart
    // from a provider.
    let generated = code.to_string();
    assert!(
        !generated.contains("insert_singleton_with_name (instance"),
        "an initializer does not register a value: {generated}"
    );
}

#[test]
fn rejects_an_initializer_with_a_return_type() {
    let item_impl = syn::parse2(quote! {
        impl MyConfiguration {
            #[init]
            fn register() -> String {
                String::new()
            }
        }
    })
    .expect("the implementation block is parsable");

    let error = generate_configuration(TokenStream2::new(), item_impl)
        .err()
        .expect("an initializer with a return type is rejected");

    assert!(
        error.to_string().contains("does not return a value"),
        "the error explains the rule: {error}"
    );
}

#[test]
fn rejects_an_initializer_in_an_auto_configuration() {
    let item_impl = syn::parse2(quote! {
        impl MyConfiguration {
            #[init]
            fn register() {}
        }
    })
    .expect("the implementation block is parsable");

    let error = generate_auto_configuration(TokenStream2::new(), item_impl)
        .err()
        .expect("`#[init]` is rejected in an auto-configuration");

    assert!(
        error.to_string().contains("only supported in a `#[configuration]`"),
        "the error explains the rule: {error}"
    );
}

#[test]
fn rejects_a_provider_without_a_return_type() {
    let item_impl = syn::parse2(quote! {
        impl MyConfiguration {
            #[provider]
            fn register() {}
        }
    })
    .expect("the implementation block is parsable");

    let error = generate_auto_configuration(TokenStream2::new(), item_impl)
        .err()
        .expect("a provider without a return type is rejected");

    assert!(
        error.to_string().contains("has to return the instance"),
        "the error explains the rule: {error}"
    );
}
