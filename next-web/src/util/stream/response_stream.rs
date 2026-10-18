//! A body that is sent as a streamed HTTP response.
//!
//! The trait separates two things a streamed response needs: the stream itself,
//! and the headers that describe it. The headers are taken first, because
//! producing the stream consumes the value that knows them, and because a body
//! that cannot be produced is answered with a response of its own — a file that
//! cannot be opened is a `500`, not a stream that fails halfway through.

use axum::{
    BoxError,
    body::{Body, Bytes},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use futures::Stream;

/// A body that can be turned into a streamed HTTP response.
pub trait ResponseStream
where
    Self: Send,
{
    /// The stream the response is sent as.
    ///
    /// The item is fixed rather than associated, because that is what
    /// [`Body::from_stream`] accepts: a chunk of bytes, or the error that ended
    /// the body early.
    type Stream: Stream<Item = Result<Bytes, BoxError>> + Send + 'static;

    /// Returns the headers the response has to carry.
    ///
    /// The headers describe the body — its content type, the name it is offered
    /// under, its length — so they are taken from the value before the stream
    /// consumes it. An empty map is allowed for a body that needs none.
    fn response_headers(&self) -> HeaderMap;

    /// Produces the stream.
    ///
    /// # Errors
    ///
    /// Returns the response to send instead when the stream cannot be produced.
    /// The failure is reported here rather than through the stream so that a
    /// missing file is a `500` rather than a `200` with a body that stops.
    fn stream(self, target_rate: usize) -> Result<Self::Stream, Response>;
}

/// Wraps a [`ResponseStream`] so it can be returned from a handler.
///
/// Sets the headers the stream asks for and sends the stream as the body.
pub struct ResponseStreamWrapper<T> {
    target_rate: usize,
    stream: T,
}

impl<T> ResponseStreamWrapper<T>
where
    T: ResponseStream,
{
    /// Wraps `stream` with the default target rate.
    pub fn with_stream(stream: T) -> Self {
        Self {
            target_rate: 2048,
            stream,
        }
    }

    /// Sets how many bytes a second the response is throttled to.
    pub fn target_rate(mut self, target_rate: usize) -> Self {
        self.target_rate = target_rate;
        self
    }
}

impl<T> IntoResponse for ResponseStreamWrapper<T>
where
    T: ResponseStream,
{
    fn into_response(self) -> axum::response::Response {
        let target_rate = if self.target_rate < 1024 {
            1024
        } else {
            self.target_rate
        };

        // The headers are taken before the stream is produced, because producing
        // it consumes the value they came from.
        let headers = self.stream.response_headers();
        let body = match self.stream.stream(target_rate) {
            Ok(body) => body,
            Err(response) => return response,
        };

        let mut builder = Response::builder().status(StatusCode::OK);
        for (name, value) in &headers {
            builder = builder.header(name, value);
        }

        builder
            .body(Body::from_stream(body))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
    }
}
