use crate::{
    chat::{model::chat_response::ChatResponse, prompt::prompt::Prompt},
    model::streaming_model::StreamingModel,
};

pub trait StreamingChatModel: StreamingModel<Prompt, ChatResponse> {}
