use std::pin::Pin;

use futures_util::StreamExt;
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
        observation::{
            chat_model_observation_convention::ChatModelObservationConvention,
            chat_model_observation_documentation::ChatModelObservationDocumentation,
            default_chat_model_observation_convention::DefaultChatModelObservationConvention,
        },
        prompt::prompt::Prompt,
    },
    model::{model::Model, model_request::ModelRequest, streaming_model::StreamingModel},
    observation::observation_documentation::ObservationDocumentation,
};

use super::api::deep_seek_api::ChatApiRespnose;

#[derive(Clone)]
pub struct DeepSeekChatModel {
    pub(crate) api: DeepSeekApi,
    pub(crate) options: DeepSeekChatOptions,
    pub(crate) observation_convention: Box<dyn ChatModelObservationConvention>,
    // todo retry
}

impl DeepSeekChatModel {
    pub fn new(api: DeepSeekApi, options: DeepSeekChatOptions) -> Self {
        Self {
            api,
            options,
            observation_convention: Box::new(DefaultChatModelObservationConvention::default()),
        }
    }

    fn crate_request(&self, prompt: Prompt, stream: bool) -> ChatCompletionRequest {
        // prompt.instructions();

        ChatCompletionRequest {
            messages: vec![],
            model: prompt.chat_options().get_model().into(),
            stream,
        }
    }

    fn to_metadata(chat_completion: &ChatCompletion, model: &str) -> ChatResponseMetadata {
        ChatResponseMetadata {
            id: chat_completion.id.to_owned(),
            model: model.into(),
            usage: Box::new(EmptyUsage),
        }
    }
}

#[async_trait]
impl Model<Prompt, ChatResponse> for DeepSeekChatModel {
    async fn call(&self, request: Prompt) -> Result<ChatResponse, BoxError> {
        let req: ChatCompletionRequest = self.crate_request(request, false);

        // observe
        // ChatModelObservationDocumentation::ChatModelOperation.observation();

        // execute
        let chat_respnose = self.api.send(&req).await?;

        let assistant_message = AssistantMessage {
            text_content: "".to_string(),
            metadata: None,
            message_type: MessageType::Assistant,
        };

        let generations = vec![Generation::new(assistant_message)];
        let chat_completion = match chat_respnose {
            ChatApiRespnose::Entity(chat_completion) => chat_completion,
            _ => return Err("Chat completion is not entity".into()),
        };

        let chat_response_meta_data = Self::to_metadata(&chat_completion, &req.model);

        Ok(ChatResponse::new(chat_response_meta_data, generations))
    }
}

#[async_trait]
impl StreamingModel<Prompt, ChatResponse> for DeepSeekChatModel {
    async fn stream(
        &self,
        request: Prompt,
    ) -> Result<
        Pin<Box<dyn futures_core::Stream<Item = Result<ChatResponse, BoxError>> + Send + 'static>>,
        BoxError,
    > {
        let req: ChatCompletionRequest = self.crate_request(request, true);
        // execute
        let chat_respnose = self.api.send(&req).await?;

        let stream = match chat_respnose {
            ChatApiRespnose::Stream(stream) => stream,
            _ => return Err("Chat completion is not stream".into()),
        };

        let stream = stream.then(move |chat_completion| {
            let model = req.model.clone();
            async move {
                let chat_completion = chat_completion.unwrap();
                Ok(ChatResponse::new(
                    Self::to_metadata(chat_completion.get(0).take().unwrap(), &model),
                    vec![],
                ))
            }
        });

        Ok(stream.boxed())
    }
}

impl ChatModel for DeepSeekChatModel {}
impl StreamingChatModel for DeepSeekChatModel {}
