use axum::http::StatusCode;
use next_web::core::traits::data_decoder::DataDecoder;
use next_web::validate::Validate;
use next_web::{
    Application, NextWebApplication,
    extract::{data::Data, validated::Validated},
    macros::bind::{get_mapping, post_mapping, singleton},
};
use serde::{Deserialize, Serialize};

use std::sync::Arc;

#[derive(Default)]
struct TestApplication;

impl Application for TestApplication {}

#[get_mapping(path = "/")]
async fn index() -> &'static str {
    "Ok"
}

#[derive(Clone, Debug, Deserialize, Validate)]
#[validate(crate = "next_web::validate")]
struct TestValidator {
    #[validate(length(min = 5, message = "Can not be empty"))]
    pub name: String,
    #[validate(range(min = 1, max = 120, message = "minimum  is 1, maximum  is 120"))]
    pub age: u8,
    #[validate(range(min = 20, max = 200, message = "minimum  is 20, maximum  is 200"))]
    pub weight: u16,
}

#[get_mapping(path = "/fromValid")]
async fn from_valid(Validated(data): Validated<TestValidator>) -> impl IntoResponse {
    assert!(data.name.len() > 4);
    assert!(data.age > 1);
    (StatusCode::OK, "Ok")
}

#[post_mapping(path = "/jsonValid")]
async fn json_valid(Validated(data): Validated<TestValidator>) -> impl IntoResponse {
    assert!(data.name.len() > 4);
    assert!(data.weight > 20);
    (StatusCode::OK, "Ok")
}

#[post_mapping(path = "/decode")]
async fn test_decode(Data(data): Data<TestData>) -> String {
    serde_json::to_string(&data).unwrap()
}

#[derive(Clone, Serialize, Deserialize)]
struct TestData {
    pub name: String,
    pub age: i32,
}

// 现状是指定这个单例名称 暂时不要改动
#[singleton(name = "defaultDataDecoder", binds=[Self::into_decoder])]
#[derive(Clone)]
pub struct TestDecoder;

impl TestDecoder {
    fn into_decoder(self) -> Arc<dyn DataDecoder> {
        Arc::new(self)
    }
}

impl DataDecoder for TestDecoder {
    fn decode(&self, data: &[u8]) -> Result<String, &'static str> {
        let d = data
            .iter()
            .filter(|&&s| s != b'\\')
            .copied()
            .collect::<Vec<_>>();
        Ok(String::from_utf8_lossy(&d).to_string())
    }
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}
