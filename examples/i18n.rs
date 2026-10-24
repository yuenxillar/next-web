use std::sync::Arc;

use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use next_web::{Application, NextWebApplication, macros::bind::get_mapping};
use next_web_context::MessageSource;

#[derive(Default)]
pub struct TestApplication;

impl Application for TestApplication {}

#[derive(Clone, serde::Serialize)]
pub struct ApiResponse {
    pub message: String,
}

impl IntoResponse for ApiResponse {
    fn into_response(self) -> axum::response::Response {
        axum::response::Response::builder()
            .status(StatusCode::OK)
            .body(self.message.into())
            .unwrap()
    }
}

#[get_mapping(path = "/message/{msg}")]
async fn req_message(
    Path(code): Path<String>,
    #[find] FindSingleton(message_source): FindSingleton<Arc<dyn MessageSource>>,
) -> impl IntoResponse {
    message_source
        .message(code.as_str(), &[], None)
        .unwrap_or("Sorry!!".into())
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}
