use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use next_web::core::ApplicationContext;
use next_web::core::context::MessageSource;
use next_web::extract::Path;
use next_web::i18n::RequestLocaleHolder;
use next_web::macros::i18n::translation;
use next_web::{
    application::Application,
    core::{async_trait, context::properties::ApplicationProperties},
    macros::bind::get_mapping,
};

#[derive(Clone, Default)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    type ErrorSolve = ();
    async fn init_middleware(
        &self,
        _ctx: &mut ApplicationContext,
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

#[translation]
#[get_mapping(path = "/message/{msg}")]
async fn req_message(
    Path(code): Path<String>,
    #[find] FindSingleton(message_source): FindSingleton<Arc<dyn MessageSource>>,
) -> impl IntoResponse {
    message_source
        .message_with_args(
            code.as_str(),
            &["Ben", "Jack", "John"],
            RequestLocaleHolder::locale_or_default(),
        )
        .unwrap_or("Sorry!!".into())
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
