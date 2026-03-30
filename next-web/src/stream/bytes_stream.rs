use axum::{
    body::{Body, Bytes},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    BoxError,
};
use futures::stream;
use next_web_core::traits::stream::into_response_stream::IntoRespnoseStream;

use crate::{
    stream::DEFAULT_CHUNK_SIZE,
    util::{local_date_time::LocalDateTime, stream_throttle::throttle_byte_stream},
};

/// A stream that sends a `Bytes` body.
pub struct BytesStream {
    /// The body.
    body: Bytes,

    /// The file name.
    file_name: Option<String>,

    /// The content type.
    content_type: Option<Box<str>>,
}

impl BytesStream {
    pub fn new<T, S>(body: T, file_name: S) -> Self
    where
        T: Into<Bytes>,
        S: Into<Option<String>>,
    {
        let body = body.into();
        let file_name = file_name.into();
        let content_type = None;

        Self {
            body,
            file_name,
            content_type,
        }
    }
}

impl BytesStream {
    /// Creates a new `BytesStreamBuilder`.
    pub fn builder() -> BytesStreamBuilder {
        BytesStreamBuilder::default()
    }
}

impl IntoRespnoseStream for BytesStream {
    fn into_response_stream(self, target_rate: usize) -> Response {
        let BytesStream {
            body,
            file_name,
            content_type,
        } = self;

        if body.is_empty() {
            return (StatusCode::OK).into_response();
        }

        let header_name = format!(
            "attachment;filename={}",
            file_name.unwrap_or(LocalDateTime::now())
        );

        let response_body = if body.len() < DEFAULT_CHUNK_SIZE * 2 {
            Body::from(body)
        } else {
            let stream = stream::unfold((body, 0usize), |(body, offset)| async move {
                if offset >= body.len() {
                    return None;
                }

                let end = (offset + DEFAULT_CHUNK_SIZE).min(body.len());
                Some((Ok::<Bytes, BoxError>(body.slice(offset..end)), (body, end)))
            });
            Body::from_stream(throttle_byte_stream(stream, target_rate))
        };

        let content_type = content_type
            .as_ref()
            .map(|s| s.as_ref())
            .unwrap_or("application/octet-stream");

        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CONTENT_DISPOSITION, header_name)
            .body(response_body)
            .unwrap()
    }
}

#[derive(Default)]
pub struct BytesStreamBuilder {
    body: Option<Bytes>,
    file_name: Option<String>,
    content_type: Option<Box<str>>,
}

impl BytesStreamBuilder {
    /// Sets the `body` field on the builder.
    pub fn body<T>(mut self, value: T) -> Self
    where
        T: Into<Bytes>,
    {
        self.body = Some(value.into());
        self
    }

    /// Sets the `file_name` field on the builder.
    pub fn file_name<T>(mut self, value: T) -> Self
    where
        T: Into<String>,
    {
        self.file_name = Some(value.into());
        self
    }

    pub fn content_type<T>(mut self, value: T) -> Self
    where
        T: Into<Box<str>>,
    {
        self.content_type = Some(value.into());
        self
    }

    /// Builds a `BytesStream` instance.
    pub fn build(self) -> Result<BytesStream, String> {
        Ok(BytesStream {
            body: self
                .body
                .ok_or_else(|| "field `body` is required".to_string())?,
            file_name: self.file_name,
            content_type: self.content_type,
        })
    }
}
