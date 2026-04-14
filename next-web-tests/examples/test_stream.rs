use std::collections::HashMap;

use axum::{body::Bytes, extract::Query, response::IntoResponse};
use next_web::{
    application::Application,
    stream::{
        bytes_stream::BytesStream, local_file_stream::LocalFileStream,
        network_file_stream::NetworkFileStream, response_stream::ResponseStream,
    },
    util::local_date_time::LocalDateTime,
};
use next_web_core::{ApplicationContext, async_trait, context::properties::ApplicationProperties};

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    type ErrorSolve = ();

    /// initialize the middleware.
    async fn init_middleware(
        &self,
        _ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    // get the application router. (open api  and private api)
    async fn application_router(&self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new()
            .route("/localFile", axum::routing::get(download_file))
            .route("/bytes", axum::routing::get(download_bytes))
            .route("/network", axum::routing::get(download_network_file))
    }
}

async fn download_file(Query(file_path): Query<String>) -> impl IntoResponse {
    // 1MB/s
    ResponseStream::with_stream(LocalFileStream(file_path)).target_rate(1024 * 1024)
}

async fn download_bytes() -> impl IntoResponse {
    // 10MB
    let bytes = Bytes::from(vec![0x97; 1024 * 1024 * 10]);

    // 10KB/s
    ResponseStream::with_stream(
        BytesStream::builder()
            .body(bytes)
            .file_name(LocalDateTime::now())
            .build()
            .unwrap(),
    )
    .target_rate(1024 * 10)
}

async fn download_network_file() -> impl IntoResponse {
    // 3KB/s
    ResponseStream::with_stream(NetworkFileStream::new(
        "http://127.0.0.1:11000/bytes",
        "GET",
        Some(HashMap::from_iter(vec![(
            "Test-Header".into(),
            "test".into(),
        )])),
    ))
    .target_rate(1024 * 6)
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
