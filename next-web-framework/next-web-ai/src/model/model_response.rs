use crate::model::{model_result::ModelResult, response_meta_data::ResponseMetadata};

pub trait ModelResponse<T, R>
where
    T: ModelResult<R>,
    R: Send,
{
    fn result(&self) -> Option<T>;

    fn results(&self) -> impl IntoIterator<Item = T>;

    fn resp_meta_data(&self) -> impl ResponseMetadata;
}
