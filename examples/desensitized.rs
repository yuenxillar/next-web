use std::borrow::Cow;

use axum::{
    body::Body,
    http::{Response, StatusCode},
    response::IntoResponse,
};
use next_web::{
    Application, NextWebApplication,
    macros::{
        bind::get_mapping,
        data::{Desensitized, GetSet},
    },
};
use next_web_core::traits::desensitized::Desensitized;

#[derive(Default)]
struct TestApplication;

impl Application for TestApplication {}

#[get_mapping(path = "/desensitized")]
async fn test() -> impl IntoResponse {
    ApiResult {
        code: 200,
        message: "Success".to_string(),
        data: Some(TestA {
            email: "test_email@xxx.com".into(),
            phone: Some("17600000000".into()),
            name: "jack".into(),
        }),
    }
}

#[derive(serde::Serialize)]
struct ApiResult<T> {
    code: i32,
    message: String,
    data: Option<T>,
}

impl<T> IntoResponse for ApiResult<T>
where
    T: serde::Serialize + Desensitized,
{
    fn into_response(mut self) -> axum::response::Response {
        self.data.as_mut().map(|val| val.desensitize());

        Response::builder()
            .status(StatusCode::OK)
            .body(Body::new(serde_json::to_string(&self.data).unwrap()))
            .unwrap()
    }
}

#[derive(Default, serde::Serialize, GetSet, Desensitized)]
struct TestA {
    #[de(email)]
    email: String,
    #[de(phone)]
    phone: Option<Box<str>>,
    #[de(name)]
    name: Cow<'static, str>,
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}
