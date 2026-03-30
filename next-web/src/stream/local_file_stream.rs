use axum::{
    body::Body,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    BoxError,
};
use futures::StreamExt;
use next_web_core::traits::stream::into_response_stream::IntoRespnoseStream;
use std::path::Path;
use tokio_util::io::ReaderStream;

use crate::{stream::DEFAULT_CHUNK_SIZE, util::stream_throttle::throttle_byte_stream};

pub struct LocalFileStream<T: AsRef<Path>>(pub T);

impl<T> IntoRespnoseStream for LocalFileStream<T>
where
    T: Send,
    T: AsRef<Path>,
{
    fn into_response_stream(self, target_rate: usize) -> axum::response::Response {
        let file_path = self.0.as_ref();

        if !file_path.exists() {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response();
        }

        let metadata = match std::fs::metadata(file_path) {
            Ok(metadata) => metadata,
            Err(_) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
            }
        };

        let std_file = match std::fs::File::open(file_path) {
            Ok(file) => file,
            Err(error) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
            }
        };

        let async_file = tokio::fs::File::from_std(std_file);
        let stream = ReaderStream::with_capacity(async_file, DEFAULT_CHUNK_SIZE)
            .map(|chunk| chunk.map_err(|error| -> BoxError { Box::new(error) }));

        let header_name = format!(
            "attachment;filename={}",
            file_path
                .file_name()
                .map(|s| s.to_str().unwrap_or_default())
                .map(|s| s.to_string())
                .unwrap_or_default()
        );

        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header(header::CONTENT_DISPOSITION, header_name)
            .header(header::CONTENT_LENGTH, metadata.len().to_string())
            .body(Body::from_stream(throttle_byte_stream(stream, target_rate)))
            .unwrap()
    }
}
