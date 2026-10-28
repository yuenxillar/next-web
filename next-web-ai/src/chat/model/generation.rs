use crate::{
    chat::{
        messages::assistant_message::AssistantMessage,
        meta_data::{
            chat_generation_meta_data::ChatGenerationMetadata,
            default_chat_generation_metadata::DefaultChatGenerationMetadata,
        },
    },
    model::{model_result::ModelResult, result_meta_data::ResultMetadata},
};

#[derive(Clone)]
pub struct Generation {
    assistant_message: AssistantMessage,
    chat_generation_metadata: Option<Box<dyn ChatGenerationMetadata>>,
}

impl Generation {
    pub fn new(assistant_message: AssistantMessage) -> Self {
        Self {
            assistant_message,
            chat_generation_metadata: None,
        }
    }
}
impl ModelResult<AssistantMessage> for Generation {
    fn output(&self) -> &AssistantMessage {
        &self.assistant_message
    }

    fn meta_data(&self) -> Box<dyn ResultMetadata> {
        self.chat_generation_metadata
            .as_ref()
            .map(|s| s.clone())
            .unwrap_or(Box::new(DefaultChatGenerationMetadata::null()))
    }
}
