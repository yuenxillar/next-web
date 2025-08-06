use crate::{
    chat::{
        messages::assistant_message::AssistantMessage, meta_data::chat_response_meta_data::ChatResponseMetadata, model::generation::Generation
    },
    model::model_response::ModelResponse,
};

pub struct ChatResponse {
    chat_response_meta_data: ChatResponseMetadata,
    generations: Vec<Generation>,
}

impl ModelResponse<Generation, AssistantMessage> for ChatResponse {
    fn result(&self) -> Generation {
        todo!()
    }

    fn results(&self) -> impl IntoIterator<Item = Generation> {
        vec![]
    }

    fn resp_meta_data(&self) -> impl crate::model::response_meta_data::ResponseMetadata {
        panic!()
    }
}
