use next_web_core::{async_trait, error::BoxError};

use crate::{
    ai::deep_seek::{
        api::deep_seek_api::{ChatCompletion, ChatCompletionRequest, DeepSeekApi},
        deep_seek_chat_options::DeepSeekChatOptions,
    },
    chat::{
        messages::{assistant_message::AssistantMessage, message_type::MessageType},
        meta_data::{chat_response_meta_data::ChatResponseMetadata, empty_usage::EmptyUsage},
        model::{
            chat_model::ChatModel, chat_response::ChatResponse, generation::Generation,
            streaming_chat_model::StreamingChatModel,
        },
        observation::chat_model_observation_convention::ChatModelObservationConvention,
        prompt::prompt::Prompt,
    },
    model::{model::Model, model_request::ModelRequest, streaming_model::StreamingModel},
};

#[derive(Clone)]
pub struct DeepSeekChatModel {
    api: DeepSeekApi,
    options: DeepSeekChatOptions,
    observation_convention: Box<dyn ChatModelObservationConvention>,
    // todo retry
}

impl DeepSeekChatModel {
    pub fn new(
        api: DeepSeekApi,
        options: DeepSeekChatOptions,
        observation_convention: Box<dyn ChatModelObservationConvention>,
    ) -> Self {
        Self {
            api,
            options,
            observation_convention,
        }
    }
    fn crate_request(&self, prompt: Prompt, stream: bool) -> ChatCompletionRequest {
        prompt.instructions();

        ChatCompletionRequest {
            messages: todo!(),
            model: todo!(),
            stream,
        }
    }

    fn _from(&self, chat_completion: ChatCompletion, model: &str) -> ChatResponseMetadata {
        ChatResponseMetadata {
            id: Default::default(),
            model: model.into(),
            usage: Box::new(EmptyUsage),
        }
    }
}

#[async_trait]
impl Model<Prompt, ChatResponse> for DeepSeekChatModel {
    async fn call(&self, request: Prompt) -> Result<ChatResponse, BoxError> {
        let req: ChatCompletionRequest = self.crate_request(request, false);

        // execute
        let chat_completion = self.api.send(&req).await?;

        let assistant_message = AssistantMessage {
            text_content: "".to_string(),
            metadata: None,
            message_type: MessageType::Assistant,
        };

        let generations = vec![Generation::new(assistant_message)];
        let chat_response_meta_data = self._from(chat_completion, &req.model);

        Ok(ChatResponse::new(chat_response_meta_data, generations))
    }
}

#[async_trait]
impl StreamingModel<Prompt, ChatResponse> for DeepSeekChatModel {
    async fn stream<S>(&self, request: Prompt) -> Result<S, BoxError>
    where
        S: futures_core::Stream<Item = ChatResponse> + Send + 'static,
    {
        let req: ChatCompletionRequest = self.crate_request(request, true);
        // execute
        let chat_completion = self.api.send(&req).await?;
        


        Ok(futures_util::stream::empty())
    }
}

impl ChatModel for DeepSeekChatModel {}
impl StreamingChatModel for DeepSeekChatModel {}
