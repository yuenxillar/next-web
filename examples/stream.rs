use std::collections::HashMap;

use axum::{body::Bytes, extract::Query};
use next_web::{
    Application, NextWebApplication,
    macros::bind::get_mapping,
    util::LocalDateTime,
    util::stream::{BytesStream, LocalFileStream, NetworkFileStream, ResponseStream},
};

#[derive(Default)]
struct TestApplication;

impl Application for TestApplication {}

#[get_mapping(path = "/localFile")]
async fn download_file(Query(file_path): Query<String>) -> impl IntoResponse {
    // 1MB/s
    ResponseStream::with_stream(LocalFileStream(file_path)).target_rate(1024 * 1024)
}

#[get_mapping(path = "/bytes")]
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

#[get_mapping(path = "/network")]
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
    NextWebApplication::<TestApplication>::default().run().await
}
