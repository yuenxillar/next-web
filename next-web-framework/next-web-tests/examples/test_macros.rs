use std::borrow::Cow;

use next_web_macro::{Builder, Desensitized, FieldName, GetSet};

/// This is a test macro
#[allow(unused)]
#[derive(Default, FieldName, GetSet, Builder)]
pub struct TestMacro {
    #[get_set(skip_get)]
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
    let test = TestMacroBuilder::builder()
    .weight(12u32)
    .x(3.2)
    .y(Some(3.5))
    .build()
    .unwrap();
}