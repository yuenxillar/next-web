use std::collections::HashMap;

use axum::{body::Bytes, response::IntoResponse};
use next_web_core::{async_trait, context::properties::ApplicationProperties, ApplicationContext};
use next_web_dev::{
    application::Application,
    stream::{
        bytes_stream::BytesStream, local_file_stream::LocalFileStream,
        network_file_stream::NetworkFileStream, response_stream::ResponseStream,
    },
};

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new()
            .route("/localFile", axum::routing::get(download_file))
            .route("/bytes", axum::routing::get(download_bytes))
            .route("/network", axum::routing::get(download_network_file))
    }
}

async fn download_file() -> impl IntoResponse {
    ResponseStream::new(LocalFileStream(
        "Please enter your large file address here. /  请在此处输入您的文件地址".into(),
    ))
}

async fn download_bytes() -> impl IntoResponse {
    // 10MB
    let bytes = Bytes::from(vec![0x01; 1024 * 1024 * 10]);
    // 10KB/s
    ResponseStream::new(BytesStream::new(bytes, Some("test.txt".into()))).target_rate(1024 * 1024)
}

async fn download_network_file() -> impl IntoResponse {
    // 3KB/s
    ResponseStream::new(NetworkFileStream::new(
        "http://127.0.0.1:11000/bytes",
        "GET",
        Some(HashMap::from_iter(vec![("Test-Header".into(), "test".into())])),
    ))
    .target_rate(1024 * 7)
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
