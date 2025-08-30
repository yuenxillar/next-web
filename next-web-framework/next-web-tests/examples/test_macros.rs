use std::borrow::Cow;

use next_web_core::traits::desensitized::Desensitized;
use next_web_macro::{Builder, Desensitized, FieldName, GetSet};

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

#[derive(Default, GetSet, Desensitized)]
struct TestA {
    s1: f32,
    s2: u32,
    s3: Vec<u32>,
    s4: Option<Vec<u32>>,
    s5: Option<TestMacro>,
    s6: Option<i32>,
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
    let mut test_a = TestA::default();
   test_a.desensitize();
}
