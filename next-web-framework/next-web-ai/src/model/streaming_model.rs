use std::pin::Pin;

use futures_core::Stream;
use next_web_core::{async_trait, error::BoxError};

#[async_trait]
pub trait StreamingModel<TReq, TResChunk>: Send {
    async fn stream(
        &self,
        request: TReq,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<TResChunk, BoxError>> + Send + 'static>>, BoxError>;
}
