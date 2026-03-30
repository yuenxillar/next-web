use axum::response::Response;

/// Convert the stream into a response stream.
pub trait IntoRespnoseStream
where
    Self: Send,
{
    fn into_response_stream(self, target_rate: usize) -> Response;
}
