use next_web_core::{async_trait, error::BoxError};

use crate::model::{
    model_request::ModelRequest, model_response::ModelResponse, model_result::ModelResult,
};

//
// pub trait Model<TReq, TRes, T, R, R1>
// where
//     TReq: ModelRequest<T>,
//     TRes: ModelResponse<R, R1>,
//     T: Send + Sync,
//     R: ModelResult<R1>,
//     R1: Send,
// {
//     async fn call(&self, request: TReq) -> Result<TRes, BoxError>;
// }

#[async_trait]
pub trait Model<TReq, TRes> {
    async fn call(&self, request: TReq) -> Result<TRes, BoxError>;
}
