use futures_core::Stream;

use crate::model::{
    model_request::ModelRequest, model_response::ModelResponse, model_result::ModelResult,
};

pub trait StreamingModel<TReq, T, TResChunk, R, R1>: Send + Sync
where
    TReq: ModelRequest<T>,
    TResChunk: ModelResponse<R, R1>,
    R: ModelResult<R1>,
    R: Send,
    R1: Send,
{
    fn stream<S>(request: TReq) -> S
    where
        S: Stream<Item = TResChunk> + Send + 'static;
}
