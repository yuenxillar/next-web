pub trait IntoRespnoseStream: Send {
    fn into_response_stream(self, target_rate: usize) -> axum::response::Response;
}
