use futures_core::Stream;
use next_web_core::{async_trait, error::BoxError};

#[async_trait]
pub trait StreamingModel<TReq, TResChunk>: Send + Sync {
    async fn stream<S>(&self, request: TReq) -> Result<S, BoxError>
    where
        S: Stream<Item = TResChunk> + Send + 'static;
}
