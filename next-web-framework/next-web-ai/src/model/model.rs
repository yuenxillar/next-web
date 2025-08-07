use next_web_core::{async_trait, error::BoxError};

use crate::model::{
    model_request::ModelRequest, model_response::ModelResponse, model_result::ModelResult,
};

#[async_trait]
pub trait Model<TReq, T, TRes, R, R1>
where
    TReq: ModelRequest<T>,
    TRes: ModelResponse<R, R1>,
    R: ModelResult<R1>,
    R: Send,
    R1: Send,
{
    async fn call(&self, request: TReq) -> Result<TRes, BoxError>;
}
