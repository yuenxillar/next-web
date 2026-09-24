use std::sync::Arc;

use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use next_web::core::ApplicationContext;
use next_web::{
    application::Application,
    core::{async_trait, context::properties::ApplicationProperties},
    macros::bind::get_mapping,
};
use next_web_context::MessageSource;

#[derive(Clone, Default)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    type ErrorSolve = ();
    async fn init_middleware(
        &self,
        _ctx: &mut dyn ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

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
    TestApplication::run().await;
}
