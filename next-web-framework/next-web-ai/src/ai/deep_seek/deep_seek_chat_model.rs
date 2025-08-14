use std::{pin::Pin, thread::spawn};

use futures_util::StreamExt;
use next_web_core::{async_trait, convert::into_box::IntoBox, error::BoxError};

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
            chat_model_observation_context::ChatModelObservationContext,
            chat_model_observation_convention::ChatModelObservationConvention,
            chat_model_observation_documentation::ChatModelObservationDocumentation,
            default_chat_model_observation_convention::DefaultChatModelObservationConvention,
            observation_convention,
        },
        prompt::{
            chat_options::{ChatOptions, DefaultChatOptions},
            prompt::Prompt,
        },
    },
    model::{model::Model, model_request::ModelRequest, streaming_model::StreamingModel},
    observation::{
        conventions::ai_provider::AiProvider,
        noop_observation_registry::NoopObservationRegistry,
        observation::Observable,
        observation_documentation::ObservationDocumentation,
        observation_registry::{ObservationRegistry, ObservationRegistryImpl},
    },
};

use super::api::deep_seek_api::ChatApiRespnose;

#[derive(Clone)]
pub struct DeepSeekChatModel<T = DefaultChatModelObservationConvention, R = NoopObservationRegistry>
{
    // pub(crate) retry_template: RetryTemplate,
    pub(crate) options: DeepSeekChatOptions,
    pub(crate) api: DeepSeekApi,
    pub(crate) observation_registry: R,
    pub(crate) observation_convention: T,
    // todo retry
}

impl DeepSeekChatModel {
    pub fn new(api: DeepSeekApi, options: DeepSeekChatOptions) -> Self {
        Self {
            api,
            options,
            observation_registry: ObservationRegistryImpl::noop(),
            observation_convention: Default::default(),
        }
    }

    fn crate_request(&self, prompt: &Prompt, stream: bool) -> ChatCompletionRequest {
        // prompt.instructions();

        ChatCompletionRequest {
            messages: vec![],
            model: prompt.chat_options().get_model().into(),
            stream,
        }
    }

    fn build_request_options(request: &ChatCompletionRequest) -> impl ChatOptions {
        DefaultChatOptions {
            model: request.model.to_string(),
            ..Default::default()
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
    async fn call(&self, prompt: Prompt) -> Result<ChatResponse, BoxError> {
        let req: ChatCompletionRequest = self.crate_request(&prompt, false);

        // observe
        let observation_context = ChatModelObservationContext::new(
            prompt,
            AiProvider::DeepSeek.to_string(),
            Self::build_request_options(&req).into_box(),
        );

        let observation_convention = self.observation_convention.clone().into_box();
        let observation = match ChatModelObservationDocumentation::ChatModelOperation.observation(
            Some(observation_convention.clone()),
            Some(observation_convention),
            observation_context.into_box(),
            self.observation_registry.clone().into_box(),
        ) {
            Ok(observation) => observation,
            Err(e) => return Err(e.into()),
        }
        .observe(Box::pin(async move { Ok(12) }));

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
        prompt: Prompt,
    ) -> Result<
        Pin<Box<dyn futures_core::Stream<Item = Result<ChatResponse, BoxError>> + Send + 'static>>,
        BoxError,
    > {
        let req: ChatCompletionRequest = self.crate_request(&prompt, true);
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
