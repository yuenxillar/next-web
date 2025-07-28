use axum::{body::Bytes, response::IntoResponse};
use next_web_core::{async_trait, context::properties::ApplicationProperties, ApplicationContext};
use next_web_dev::{
    application::Application,
    stream::{
        bytes_stream::BytesStream, local_file_stream::LocalFileStream,
        response_stream::ResponseStream,
    },
};

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&mut self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new()
            .route("/localFile", axum::routing::get(download_file))
            .route("/bytes", axum::routing::get(download_bytes))
            .route("/network", axum::routing::get(download_network_file))
    }
}

async fn download_file() -> impl IntoResponse {
    ResponseStream::new(LocalFileStream(
        "Please enter your large file address here. /  请在此处输入您的大文件地址".into(),
    ))
}

async fn download_bytes() -> impl IntoResponse {
    // 100MB
    let bytes = Bytes::from(vec![0x01; 1024 * 1024 * 100]);
    // 10KB/s
    ResponseStream::new(BytesStream::new(bytes, Some("test.txt".into()))).target_rate(1024 * 10)
}


async fn download_network_file() -> impl IntoResponse {
    let bytes = Bytes::from(vec![0x01; 1024 * 1024 * 100]);
    ResponseStream::new(BytesStream::new(bytes, Some("test.txt".into()))).target_rate(1024 * 10)
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
