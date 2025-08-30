use axum::response::IntoResponse;
use next_web_core::traits::stream::into_response_stream::IntoRespnoseStream;

pub struct ResponseStream<T> {
    target_rate: usize,
    stream: T,
}

impl<T> ResponseStream<T>
where
    T: IntoRespnoseStream,
{
    pub fn new(stream: T) -> Self {
        Self {
            target_rate: 2048,
            stream,
        }
    }

    pub fn target_rate(mut self, target_rate: usize) -> Self {
        self.target_rate = target_rate;
        self
    }
}

impl<T> IntoResponse for ResponseStream<T>
where
    T: IntoRespnoseStream,
{
    fn into_response(self) -> axum::response::Response {
        // 凡事留一线 日后好相见！
        let target_rate = if self.target_rate < 1024 {
            1024
        } else {
            self.target_rate
        };
        self.stream.into_response_stream(target_rate)
    }
}