use std::borrow::Cow;

use next_web_macro::{Builder, FieldName, GetSet};

/// This is a test macro

#[derive(Default, GetSet, Builder, FieldName)]
pub struct TestMacro {
    #[get_set(skip_get)]
    description: String,
    weight: u32,
    x: f32,
    #[get_set(skip_set)]
    y: Option<f32>,
}

#[derive(Default, GetSet)]
struct TestA {
    s1: f32,
    s2: u32,
    s3: Vec<u32>,
    s4: Option<Vec<u32>>,
    s5: Option<TestMacro>,
    s6: Option<i32>,
    s7: Box<str>,
    s8: Cow<'static, str>,
    s9: String,
    s10: &'static str,
    s11: bool,
}

fn main() {
    let test_a = TestA::default();
}
