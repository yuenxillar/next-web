use std::ops::Deref;

use crate::chat::{
    model::chat_response::ChatResponse,
    observation::model_observation_context::ModelObservationContext,
    prompt::{chat_options::ChatOptions, prompt::Prompt},
};

#[derive(Clone)]
pub struct ChatModelObservationContext {
    // request_options: Box<dyn ChatOptions>,
    model_observation_context: ModelObservationContext<Prompt, ChatResponse>,
}

impl ChatModelObservationContext {
    // pub fn request_options(&self) -> &Box<dyn ChatOptions> {
    //     &self.request_options
    // }
}

impl Deref for ChatModelObservationContext {
    type Target = ModelObservationContext<Prompt, ChatResponse>;

    fn deref(&self) -> &Self::Target {
        &self.model_observation_context
    }
}
