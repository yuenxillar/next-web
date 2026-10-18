use super::ResponseStream;
use axum::{
    BoxError,
    body::Bytes,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use futures::{StreamExt, stream};

use crate::util::{
    LocalDateTime, stream::DEFAULT_CHUNK_SIZE, stream_throttle::throttle_byte_stream,
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

impl ResponseStream for BytesStream {
    type Stream = futures::stream::BoxStream<'static, Result<Bytes, BoxError>>;

    /// Describes the body: what it is, and the name it is offered under.
    fn response_headers(&self) -> axum::http::HeaderMap {
        let mut headers = axum::http::HeaderMap::new();

        let content_type = self
            .content_type
            .as_deref()
            .unwrap_or("application/octet-stream");
        if let Ok(value) = axum::http::HeaderValue::from_str(content_type) {
            headers.insert(header::CONTENT_TYPE, value);
        }

        let file_name = self
            .file_name
            .as_deref()
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(LocalDateTime::now);
        if let Ok(value) =
            axum::http::HeaderValue::from_str(&format!("attachment;filename={file_name}"))
        {
            headers.insert(header::CONTENT_DISPOSITION, value);
        }

        headers
    }

    fn stream(self, target_rate: usize) -> Result<Self::Stream, Response> {
        let BytesStream { body, .. } = self;

        // An empty body is answered without a body at all, rather than as a stream
        // that ends immediately.
        if body.is_empty() {
            return Err(StatusCode::OK.into_response());
        }

        // A small body is sent as one chunk: splitting it would add a future per
        // chunk without anything to gain.
        //
        // NOTE: `stream::unfold` produces a `!Unpin` future internally, so the
        // resulting stream is not `Unpin`. Boxing it with `.boxed()` makes it a
        // `BoxStream` (always `Unpin`), which is required by `Body::from_stream`
        // and by `throttle_byte_stream`.
        if body.len() < DEFAULT_CHUNK_SIZE * 2 {
            return Ok(stream::once(async move { Ok::<Bytes, BoxError>(body) }).boxed());
        }

        let stream = stream::unfold((body, 0usize), |(body, offset)| async move {
            if offset >= body.len() {
                return None;
            }
            let end = (offset + DEFAULT_CHUNK_SIZE).min(body.len());
            Some((Ok::<Bytes, BoxError>(body.slice(offset..end)), (body, end)))
        })
        .boxed();

        match target_rate {
            0 => Ok(stream),
            rate => Ok(throttle_byte_stream(stream, rate).boxed()),
        }
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
