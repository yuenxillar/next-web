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
        data::{Desensitize, GetSet},
    },
    wrapper::Desensitized,
};

#[derive(Default)]
struct TestApplication;

impl Application for TestApplication {}

#[get_mapping(path = "/desensitized")]
async fn test() -> impl IntoResponse {
    let value = TestA {
        email: "test_email@163.com".into(),
        phone: Some("17600000000".into()),
        name: "jack".into(),
        plate: "HuA12345".into(),
    };

    Desensitized::with(value).to_response(ApiResult::ok)
}

#[derive(serde::Serialize)]
struct ApiResult<T> {
    code: i32,
    message: String,
    data: Option<T>,
}

impl<T> ApiResult<T> {
    pub fn ok(data: T) -> Self {
        Self {
            code: 200,
            message: "success".to_owned(),
            data: Some(data),
        }
    }
}

impl<T> IntoResponse for ApiResult<T>
where
    T: serde::Serialize,
{
    fn into_response(self) -> axum::response::Response {
        Response::builder()
            .status(StatusCode::OK)
            .body(Body::new(serde_json::to_string(&self.data).unwrap()))
            .unwrap()
    }
}

/// The data of the example, with a desensitizer of the framework for a phone
/// number and a desensitizer of the application for a plate.
#[derive(Default, serde::Serialize, GetSet, Desensitize)]
struct TestA {
    #[de(email)]
    email: String,
    #[de(phone)]
    phone: Option<Box<str>>,
    #[de(name)]
    name: Cow<'static, str>,
    #[de(with = de_license_plate)]
    plate: String,
}

/// Masks every character of a license plate but the first two.
fn de_license_plate(value: &str) -> String {
    format!("{}***", value.chars().take(2).collect::<String>())
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}
