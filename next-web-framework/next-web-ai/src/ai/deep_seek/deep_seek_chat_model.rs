use futures_core::stream::BoxStream;
use futures_util::StreamExt;
use next_web_core::{async_trait, convert::into_box::IntoBox, error::BoxError};

use crate::{
    ai::deep_seek::{
        api::deep_seek_api::{
            ChatCompletion, ChatCompletionMessage, ChatCompletionRequest, DeepSeekApi, DefaultUsage,
        },
        deep_seek_chat_options::DeepSeekChatOptions,
    },
    chat::{
        messages::{assistant_message::AssistantMessage, message_type::MessageType},
        meta_data::{
            chat_response_meta_data::ChatResponseMetadata, empty_usage::EmptyUsage, usage::Usage,
        },
        model::{
            chat_model::ChatModel, chat_response::ChatResponse, generation::Generation,
            streaming_chat_model::StreamingChatModel,
        },
        observation::{
            chat_model_observation_context::ChatModelObservationContext,
            chat_model_observation_documentation::ChatModelObservationDocumentation,
            default_chat_model_observation_convention::DefaultChatModelObservationConvention,
        },
        prompt::{
            chat_options::{ChatOptions, DefaultChatOptions},
            prompt::Prompt,
        },
    },
    model::{model::Model, model_request::ModelRequest, streaming_model::StreamingModel},
    observation::{
        conventions::ai_provider::AiProvider, noop_observation_registry::NoopObservationRegistry,
        observation::Observable, observation_documentation::ObservationDocumentation,
        observation_registry::ObservationRegistryImpl,
    },
};

use super::api::deep_seek_api::ChatApiRespnose;

#[derive(Clone)]
pub struct DeepSeekChatModel<T = DefaultChatModelObservationConvention, R = NoopObservationRegistry>
{
    // pub(crate) retry_template: RetryTemplate,
    pub(crate) options: Option<DeepSeekChatOptions>,
    pub(crate) api: DeepSeekApi,
    pub(crate) observation_registry: R,
    pub(crate) observation_convention: T,
    // todo retry
}

impl DeepSeekChatModel {
    pub fn new(api: DeepSeekApi, options: Option<DeepSeekChatOptions>) -> Self {
        Self {
            api,
            options,
            observation_registry: ObservationRegistryImpl::noop(),
            observation_convention: Default::default(),
        }
    }

    fn crate_request(
        &self,
        prompt: &Prompt,
        stream: bool,
    ) -> Result<ChatCompletionRequest, &'static str> {
        let (system_messages, user_messages) = prompt.instructions().iter().fold(
            (Vec::new(), Vec::new()),
            |(mut sys, mut user), s| {
                let msg = ChatCompletionMessage::new(s.message_type().as_ref(), s.text());
                match s.message_type() {
                    MessageType::System => sys.push(msg),
                    _ => user.push(msg),
                }
                (sys, user)
            },
        );

        if system_messages.len() > 1 {
            return Err("Only one system message is allowed in the prompt");
        }

        let system_message = system_messages.first();

        let request = ChatCompletionRequest {
            messages: user_messages,
            model: prompt.chat_options().get_model().into(),
            stream,
            temperature: None,
        };

        // if (this.defaultOptions != null) {
		// 	request = ModelOptionsUtils.merge(this.defaultOptions, request, ChatCompletionRequest.class);
		// }

		// if (prompt.getOptions() != null) {
		// 	var updatedRuntimeOptions = ModelOptionsUtils.copyToTarget(prompt.getOptions(), ChatOptions.class,
		// 			QianFanChatOptions.class);
		// 	request = ModelOptionsUtils.merge(updatedRuntimeOptions, request, ChatCompletionRequest.class);
		// }

        if let Some(options) = self.options.as_ref() {
            
        }

        if let Some(options) = prompt.options() {

        }

        Ok(request)
    }

    fn build_request_options(request: &ChatCompletionRequest) -> impl ChatOptions {
        DefaultChatOptions {
            model: request.model.to_string(),
            ..Default::default()
        }
    }

    fn to_metadata(chat_completion: &ChatCompletion, model: &str) -> ChatResponseMetadata {
        let usage: Box<dyn Usage> = chat_completion
            .usage
            .as_ref()
            .map(|u| Self::default_usage(u).into_boxed() as Box<dyn Usage>)
            .unwrap_or_else(|| EmptyUsage.into_boxed());

        ChatResponseMetadata {
            id: chat_completion.id.to_owned(),
            model: model.into(),
            usage,
        }
    }

    fn default_usage(usage: &super::api::deep_seek_api::Usage) -> DefaultUsage {
        DefaultUsage {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
        }
    }
}

#[async_trait]
impl Model<Prompt, ChatResponse> for DeepSeekChatModel {
    async fn call(&self, prompt: Prompt) -> Result<ChatResponse, BoxError> {
        let req: ChatCompletionRequest = self.crate_request(&prompt, false)?;

        // observe
        let mut observation_context = ChatModelObservationContext::new(
            prompt,
            AiProvider::DeepSeek.to_string(),
            Self::build_request_options(&req).into_boxed(),
        );

        let observation_convention = self.observation_convention.clone().into_boxed();
        match ChatModelObservationDocumentation::ChatModelOperation.observation(
            Some(observation_convention.clone()),
            Some(observation_convention),
            observation_context.clone(),
            self.observation_registry.clone().into_boxed(),
        ) {
            Ok(observation) => observation,
            Err(e) => return Err(e.into()),
        }
        .observe(async {
            // execute
            let chat_respnose = self.api.send(&req).await?;

            let chat_completion = match chat_respnose {
                ChatApiRespnose::Entity(chat_completion) => chat_completion,
                _ => return Err("Chat completion is not entity".into()),
            };

            let text_content = chat_completion
                .choices
                .first()
                .and_then(|s| s.message.as_ref().and_then(|s1| Some(s1.content.clone())))
                .unwrap_or_default()
                .to_string();

            let assistant_message = AssistantMessage {
                text_content,
                metadata: None,
                message_type: MessageType::Assistant,
            };

            let generations = vec![Generation::new(assistant_message)];
            let chat_response_meta_data = Self::to_metadata(&chat_completion, &req.model);

            let chat_response = ChatResponse::new(chat_response_meta_data, generations);
            observation_context.set_response(chat_response.clone());

            Ok(chat_response)
        })
        .await
    }
}

#[async_trait]
impl StreamingModel<Prompt, ChatResponse> for DeepSeekChatModel {
    async fn stream(
        &self,
        prompt: Prompt,
    ) -> Result<BoxStream<'static, Result<ChatResponse, BoxError>>, BoxError> {
        let req: ChatCompletionRequest = self.crate_request(&prompt, true)?;
        // execute
        let chat_respnose = self.api.send(&req).await?;

        let stream = match chat_respnose {
            ChatApiRespnose::Stream(stream) => stream,
            _ => return Err("Chat completion is not stream".into()),
        };

        let stream = stream.then(move |chat_completion| {
            let model = req.model.clone();
            async move {
                let chat_completion = chat_completion?;

                let generations: Vec<Generation> = chat_completion
                    .iter()
                    .flat_map(|response| &response.choices)
                    .filter_map(|choice| choice.delta.as_ref())
                    .map(|delta| {
                        Generation::new(AssistantMessage {
                            text_content: delta.content.to_string(),
                            metadata: None,
                            message_type: MessageType::Assistant,
                        })
                    })
                    .collect();

                Ok(ChatResponse::new(
                    Self::to_metadata(chat_completion.last().unwrap(), &model),
                    generations,
                ))
            }
        });

        Ok(stream.boxed())
    }
}

impl ChatModel for DeepSeekChatModel {}
impl StreamingChatModel for DeepSeekChatModel {}
