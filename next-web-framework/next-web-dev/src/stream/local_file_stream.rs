use axum::{
    body::Body,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use futures::StreamExt;
use next_web_core::traits::stream::into_response_stream::IntoRespnoseStream;
use std::{
    path::Path,
    time::{Duration, Instant},
};

use crate::stream::DEFAULT_CHUNK_SIZE;

pub struct LocalFileStream(pub String);

impl IntoRespnoseStream for LocalFileStream {
    fn into_response_stream(self, target_rate: usize) -> axum::response::Response {
        let file_path = Path::new(self.0.as_str());

        if !file_path.exists() {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response();
        }

        if let Ok(metadata) = std::fs::metadata(file_path) {
            if (metadata.len() as usize) < DEFAULT_CHUNK_SIZE * 2 {
                return match std::fs::read(file_path) {
                    Ok(data) => (StatusCode::OK, data.into_response()).into_response(),
                    Err(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
                    }
                };
            }
        }

        let std_file = std::fs::File::open(file_path);

        if let Err(error) = std_file {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }

        let async_file = tokio::fs::File::from_std(std_file.unwrap());

        let start_time = Instant::now();
        let mut bytes_sent = 0;

        let stream = tokio_util::io::ReaderStream::with_capacity(async_file, DEFAULT_CHUNK_SIZE)
            .then(move |chunk| {
                let chunk_len = chunk.as_ref().map(|s| s.len()).unwrap_or_default();
                let now = Instant::now();
                let elapsed = now.duration_since(start_time);
                let expected_time = Duration::from_secs_f64(bytes_sent as f64 / target_rate as f64);

                let delay = if expected_time > elapsed {
                    expected_time - elapsed
                } else {
                    Duration::from_secs(0)
                };

                bytes_sent += chunk_len;

                async move {
                    if !delay.is_zero() {
                        tokio::time::sleep(delay).await;
                    }

                    let result: Result<axum::body::Bytes, axum::BoxError> =
                        chunk.map_err(Into::into);
                    result
                }
            });

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
            .body(Body::from_stream(stream))
            .unwrap()
    }
}
