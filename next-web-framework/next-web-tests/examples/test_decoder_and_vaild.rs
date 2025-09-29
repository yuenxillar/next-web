use axum::{http::StatusCode, response::IntoResponse};
use next_web_core::{
    async_trait, context::properties::ApplicationProperties, traits::data_decoder::DataDecoder,
    ApplicationContext,
};
use next_web_dev::{
    application::Application,
    extract::{data::Data, validated::Validated},
    Singleton,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use std::sync::Arc;

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new()
            .route("/", axum::routing::get(|| async move { "Ok" }))
            .route("/decode", axum::routing::post(test_decode))
            .route("/fromValid", axum::routing::get(from_valid))
            .route("/jsonValid", axum::routing::post(json_valid))
    }
}

#[derive(Clone, Debug, Deserialize, Validate)]
struct TestValidator {
    #[validate(length(min = 5, message = "Can not be empty"))]
    pub name: String,
    #[validate(range(min = 1, max = 120, message = "minimum  is 1, maximum  is 120"))]
    pub age: u8,
    #[validate(range(min = 20, max = 200,message = "minimum  is 20, maximum  is 200"))]
    pub weight: u16,
}

async fn from_valid(Validated(data): Validated<TestValidator>) -> impl IntoResponse {
    assert!(data.name.len() > 4);
    assert!(data.age > 1);
    (StatusCode::OK, "Ok")
}

async fn json_valid(Validated(data): Validated<TestValidator>) -> impl IntoResponse {
    assert!(data.name.len() > 4);
    assert!(data.weight > 20);
    (StatusCode::OK, "Ok")
}

async fn test_decode(Data(data): Data<TestData>) -> String {
    serde_json::to_string(&data).unwrap()
}

#[derive(Clone, Serialize, Deserialize)]
struct TestData {
    pub name: String,
    pub age: i32,
}


// 现状是指定这个单例名称 暂时不要改动
#[Singleton(name = "defaultDataDecoder", binds=[Self::into_decoder])]
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
    TestApplication::run().await;
}
