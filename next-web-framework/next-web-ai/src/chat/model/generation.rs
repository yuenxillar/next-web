use crate::{
    chat::{
        messages::assistant_message::AssistantMessage,
        meta_data::chat_generation_meta_data::ChatGenerationMetadata,
    },
    model::model_result::ModelResult,
};

#[derive(Clone)]
pub struct Generation {
    assistant_message: AssistantMessage,
    chat_generation_metadata: Box<dyn ChatGenerationMetadata>,
}

impl ModelResult<AssistantMessage> for Generation
{
    fn output(&self) -> AssistantMessage {
        todo!()
    }

    fn meta_data(&self) -> impl crate::model::result_meta_data::ResultMetadata {
        todo!()
    }
}
