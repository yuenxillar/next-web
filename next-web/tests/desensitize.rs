//! The `#[de]` attribute of the `Desensitize` derive.

use std::borrow::Cow;

use next_web::macros::data::Desensitize;
use next_web_core::util::Desensitize;

/// Masks every character of a plate but the first two.
fn de_plate(value: &str) -> String {
    format!("{}***", value.chars().take(2).collect::<String>())
}

/// Masks every character of an identifier but the last four.
fn de_id(value: &str) -> Cow<'static, str> {
    let hidden = value.chars().count().saturating_sub(4);
    let visible: String = value.chars().skip(hidden).collect();

    Cow::Owned(format!("{}{visible}", "*".repeat(hidden)))
}

/// A structure that masks its fields with a type of the framework and with
/// functions of the application.
#[derive(Desensitize)]
struct Person {
    /// A type of the framework, for a field of `String`.
    #[de(phone)]
    phone: String,

    /// A type of the framework, for a field of `Option<String>`.
    #[de(email)]
    email: Option<String>,

    /// A function of the application, named with `with`.
    #[de(with = de_plate)]
    plate: String,

    /// A function of the application, named by its path.
    #[de(de_id)]
    id: Cow<'static, str>,

    /// A field without `de` is left as it is.
    name: String,
}

/// Returns a person of the test.
fn a_person() -> Person {
    Person {
        phone: "17600000000".to_owned(),
        email: Some("test_email@xxx.com".to_owned()),
        plate: "沪A12345".to_owned(),
        id: Cow::Borrowed("110101199001011234"),
        name: "jack".to_owned(),
    }
}

#[test]
fn a_type_of_the_framework_masks_a_string_field() {
    let mut person = a_person();
    person.desensitize();

    assert_eq!(person.phone, "176****0000");
}

#[test]
fn a_type_of_the_framework_masks_the_value_of_an_option() {
    let mut person = a_person();
    person.desensitize();

    assert_eq!(person.email.as_deref(), Some("te****@xxx.com"));

    let mut absent = a_person();
    absent.email = None;
    absent.desensitize();

    assert_eq!(absent.email, None);
}

#[test]
fn a_function_of_the_application_masks_a_field() {
    let mut person = a_person();
    person.desensitize();

    assert_eq!(person.plate, "沪A***");
    assert_eq!(person.id, Cow::Borrowed("**************1234"));
}

#[test]
fn a_field_without_the_attribute_is_left_as_it_is() {
    let mut person = a_person();
    person.desensitize();

    assert_eq!(person.name, "jack");
}

#[test]
fn the_containers_of_a_structure_mask_their_contents() {
    let mut people = vec![Some(a_person())];
    people.desensitize();

    let person = people.pop().flatten().expect("the person is present");
    assert_eq!(person.phone, "176****0000");
    assert_eq!(person.plate, "沪A***");
}
