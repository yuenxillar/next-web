use axum::http::StatusCode;
use next_web::extract::ValidatedJson;
use next_web::validator::Validate;
use next_web::{
    Application, NextWebApplication, extract::validated::ValidatedForm, macros::bind::post_mapping,
};
use serde::Deserialize;

#[derive(Default)]
struct TestApplication;

impl Application for TestApplication {}

#[derive(Clone, Debug, Deserialize, Validate)]
#[validate(crate = "next_web::validator")]
struct TestValidator {
    #[validate(length(min = 5, message = "Can not be empty"))]
    pub name: String,
    #[validate(range(min = 1, max = 120, message = "minimum  is 1, maximum  is 120"))]
    pub age: u8,
    #[validate(range(min = 20, max = 200, message = "minimum  is 20, maximum  is 200"))]
    pub weight: u16,
}

#[post_mapping(path = "/fromValid")]
async fn from_valid(ValidatedForm(data): ValidatedForm<TestValidator>) -> impl IntoResponse {
    assert!(data.name.len() > 4);
    assert!(data.age > 1);
    (StatusCode::OK, "Ok")
}

#[post_mapping(path = "/jsonValid")]
async fn json_valid(ValidatedJson(data): ValidatedJson<TestValidator>) -> impl IntoResponse {
    assert!(data.name.len() > 4);
    assert!(data.weight > 20);
    (StatusCode::OK, "Ok")
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}
