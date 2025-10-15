use std::time::{Duration, Instant};

use axum::{
    body::{Body, Bytes},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    BoxError,
};
use futures::{stream, StreamExt};
use next_web_core::traits::stream::into_response_stream::IntoRespnoseStream;

use crate::{stream::DEFAULT_CHUNK_SIZE, util::local_date_time::LocalDateTime};

pub struct BytesStream {
    body: Bytes,
    file_name: Option<String>,
}


impl BytesStream {
    
    pub fn new(body: Bytes, file_name: Option<String>) -> Self {
        Self { body, file_name }
    }
}

impl IntoRespnoseStream for BytesStream {
    fn into_response_stream(self, target_rate: usize) -> Response {
        if self.body.is_empty() {
            return (StatusCode::OK, "").into_response();
        }

        if self.body.len() < DEFAULT_CHUNK_SIZE * 2 {
            return (StatusCode::OK, self.body).into_response();
        }

        let chunks: Vec<Bytes> = self
            .body
            .chunks(DEFAULT_CHUNK_SIZE)
            .map(|slice| Bytes::copy_from_slice(slice))
            .collect();

        let start_time = Instant::now();
        let mut bytes_sent = 0;

        let stream = stream::iter(chunks).then(move |chunk| {
            let chunk_len = chunk.len();
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
                Ok::<_, BoxError>(chunk)
            }
        });

        let header_name = format!(
            "attachment;filename={}",
            self.file_name.map(|s| s).unwrap_or(LocalDateTime::now())
        );

        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header(header::CONTENT_DISPOSITION, header_name)
            .body(Body::from_stream(stream))
            .unwrap()
    }
}
