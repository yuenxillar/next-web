use crate::{
    chat::{
        messages::assistant_message::AssistantMessage,
        meta_data::chat_response_meta_data::ChatResponseMetadata, model::generation::Generation,
    },
    model::model_response::ModelResponse,
};

#[derive(Clone)]
pub struct ChatResponse {
    pub(crate) chat_response_meta_data: ChatResponseMetadata,
    pub(crate) generations: Vec<Generation>,
}

impl ChatResponse {
    pub fn new(
        chat_response_meta_data: ChatResponseMetadata,
        generations: Vec<Generation>,
    ) -> Self {
        Self {
            chat_response_meta_data,
            generations,
        }
    }
}

impl ModelResponse<Generation, AssistantMessage> for ChatResponse {
    fn result(&self) -> Option<Generation> {
        self.generations
            .is_empty()
            .then(|| self.generations.first().cloned())
            .unwrap_or(None)
    }

    fn results(&self) -> impl IntoIterator<Item = Generation> {
        self.generations.to_owned()
    }

    fn resp_meta_data(&self) -> impl crate::model::response_meta_data::ResponseMetadata {
        self.chat_response_meta_data.clone()
    }
}
