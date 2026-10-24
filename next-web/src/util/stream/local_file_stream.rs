//! A file on the local file system, sent as a streamed response.

use super::ResponseStream;
use axum::{
    BoxError,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use futures::{StreamExt, stream::BoxStream};
use std::path::Path;
use tokio_util::io::ReaderStream;

use crate::util::{stream::DEFAULT_CHUNK_SIZE, stream_throttle::throttle_byte_stream};

/// A file on the local file system.
pub struct LocalFileStream<T: AsRef<Path>>(pub T);

impl<T> ResponseStream for LocalFileStream<T>
where
    T: Send,
    T: AsRef<Path>,
{
    type Stream = BoxStream<'static, Result<axum::body::Bytes, BoxError>>;

    /// Describes the file: it is an octet stream, offered under the name it has on
    /// disk, with the length the file system reports.
    ///
    /// A file whose metadata cannot be read still gets the first two headers; the
    /// length is simply left out, and [`ResponseStream::stream`] answers with the
    /// error response instead.
    fn response_headers(&self) -> axum::http::HeaderMap {
        let file_path = self.0.as_ref();
        let mut headers = axum::http::HeaderMap::new();

        headers.insert(
            header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/octet-stream"),
        );

        let file_name = file_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if let Ok(value) =
            axum::http::HeaderValue::from_str(&format!("attachment;filename={file_name}"))
        {
            headers.insert(header::CONTENT_DISPOSITION, value);
        }

        if let Ok(metadata) = std::fs::metadata(file_path) {
            if let Ok(value) = axum::http::HeaderValue::from_str(&metadata.len().to_string()) {
                headers.insert(header::CONTENT_LENGTH, value);
            }
        }

        headers
    }

    /// Opens the file and streams it.
    ///
    /// # Errors
    ///
    /// Returns a `500` response when the file does not exist, its metadata cannot
    /// be read, or it cannot be opened — the failure is reported here, before the
    /// response starts, rather than as a body that stops halfway.
    fn stream(self, target_rate: usize) -> Result<Self::Stream, Response> {
        let file_path = self.0.as_ref();

        if !file_path.exists() {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response());
        }

        if std::fs::metadata(file_path).is_err() {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response());
        }

        let std_file = match std::fs::File::open(file_path) {
            Ok(file) => file,
            Err(error) => {
                return Err(
                    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
                );
            }
        };

        let async_file = tokio::fs::File::from_std(std_file);
        let stream = ReaderStream::with_capacity(async_file, DEFAULT_CHUNK_SIZE)
            .map(|chunk| chunk.map_err(|error| -> BoxError { Box::new(error) }))
            .boxed();

        match target_rate {
            0 => Ok(stream),
            rate => Ok(throttle_byte_stream(stream, rate).boxed()),
        }
    }
}
