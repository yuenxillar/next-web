use futures_core::stream::BoxStream;

use next_web_core::{async_trait, error::BoxError};

#[async_trait]
pub trait StreamingModel<TReq, TResChunk>: Send {
    async fn stream(
        &self,
        request: TReq,
    ) -> Result<BoxStream<'static, Result<TResChunk, BoxError>>, BoxError>;
}
