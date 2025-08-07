use next_web_core::DynClone;

use crate::{
    chat::{model::chat_response::ChatResponse, prompt::prompt::Prompt},
    model::{
        model::Model, model_request::ModelRequest, model_response::ModelResponse,
        model_result::ModelResult,
    },
};

pub trait ChatModel
where
    Self: Model<Prompt, ChatResponse>,
    Self: DynClone + Send + Sync + 'static,
{
    
}

// pub trait ChatModel<T, R, R1>:
//     Model<Prompt, ChatResponse, T, R, R1> + DynClone + Send + Sync + 'static
// where
//     Prompt: ModelRequest<T> + Send + Sync + 'static, // Added this bound
//     ChatResponse: ModelResponse<R, R1> + Send + Sync + 'static, // And this one
//     T: Send + Sync + 'static,
//     R: ModelResult<R1> + 'static,
//     R1: Send + Sync + 'static,
// {
//     // ChatModel specific methods
// }

next_web_core::clone_trait_object!(ChatModel);
