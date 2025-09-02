use std::borrow::Cow;

use next_web_core::traits::desensitized::Desensitized;
use next_web_macros::{Builder, Desensitized, FieldName, GetSet};

/// This is a test macro
#[allow(unused)]
#[derive(Debug, Default, FieldName, GetSet, Builder)]
pub struct TestMacro {
    #[get_set(skip_get)]
    #[builder(into)]
    description: String,
    #[builder(default, into)]
    weight: u32,
    #[builder(default = "default_x")]
    x: f32,
    #[get_set(skip)]
    y: Option<f32>,
}

fn default_x() -> f32 {
    2.0
}

#[allow(unused)]
#[derive(Default, GetSet, Desensitized, FieldName)]
struct TestA {
    s1: f32,
    s2: u32,
    s3: Vec<u32>,
    s4: Option<Vec<u32>>,
    s5: Option<TestMacro>,
    s6: Option<i32>,
    #[get_set(skip)]
    s7: Box<str>,
    s8: Cow<'static, str>,
    #[de(email)]
    s9: String,
    #[de(phone)]
    s10: Option<Box<str>>,
    #[de(phone)]
    s11: Cow<'static, str>,
}

fn main() {
    // Getters and setters
    let mut test_a = TestA::default();
    test_a.get_s4();
    test_a.set_s4(None);


    // Field name
    assert_eq!(TestA::field_s2(), "s2");
    assert_eq!(TestA::field_s5(), "s5");
    assert_eq!(TestA::field_s7(), "s7");
    assert_eq!(TestA::field_s8(), "s8");
    assert_eq!(TestA::field_s9(), "s9");


    // Desensitized
    test_a.desensitize();
    

    // Builder
    let test = TestMacroBuilder::builder()
        .weight(32u32)
        .description("this is a test macro")
        .x(1.2)
        .y(2.3)
        .build();

    match test {
        Ok(val) => println!("{:?}", val),
        Err(msg) => eprintln!("Error: {}", msg),
    }
}
