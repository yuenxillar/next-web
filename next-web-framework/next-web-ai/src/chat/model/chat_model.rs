use next_web_core::DynClone;

use crate::{
    chat::{model::chat_response::ChatResponse, prompt::prompt::Prompt},
    model::{model::Model, model_result::ModelResult},
};

pub trait ChatModel<T, R, R1>
where
    Self: Model<Prompt, T, ChatResponse, R, R1>,
    Self: DynClone + Send + Sync + 'static,
    R: ModelResult<R1> + Send,
    R1: Send,
{

}

next_web_core::clone_trait_object!(ChatModel);
