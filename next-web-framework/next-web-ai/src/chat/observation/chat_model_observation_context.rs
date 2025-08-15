use std::ops::{Deref, DerefMut};

use crate::{chat::{
    model::chat_response::ChatResponse,
    observation::{
        ai_operation_metadata::AiOperationMetadata,
        conventions::ai_operation_type::AiOperationType,
        model_observation_context::ModelObservationContext,
    },
    prompt::{chat_options::ChatOptions, prompt::Prompt},
}, observation::observation::Context};

#[derive(Clone)]
pub struct ChatModelObservationContext {
    request_options: Box<dyn ChatOptions>,
    model_observation_context: ModelObservationContext<Prompt, ChatResponse>,
}

impl ChatModelObservationContext {
    pub fn new(
        prompt: Prompt,
        provider: impl Into<String>,
        request_options: Box<dyn ChatOptions>,
    ) -> Self {
        let operation_metadata = AiOperationMetadata {
            operation_type: AiOperationType::Chat.to_string(),
            provider: provider.into(),
        };
        Self {
            model_observation_context: ModelObservationContext::new(prompt, operation_metadata),
            request_options,
        }
    }
}


impl ChatModelObservationContext {

     pub fn request_options(&self) -> & dyn ChatOptions  {
        self.request_options.as_ref()
    }
}

impl Context for ChatModelObservationContext {
    fn set_parent_from_current_observation(&mut self, registry: &dyn crate::observation::observation_registry::ObservationRegistry) {
        
    }

    fn add_low_cardinality_key_values(&mut self, key_values: crate::util::key_values::KeyValues<Box<dyn crate::util::key_value::KeyValue>>) {
        
    }

    fn set_name(&mut self, name: &str) {
        self.name = Some(name.to_string());
    }

    fn set_contextual_name(&mut self, contextual_name: &str) {
        self.contextual_name = Some(contextual_name.to_string());
    }
}

impl Deref for ChatModelObservationContext {
    type Target = ModelObservationContext<Prompt, ChatResponse>;

    fn deref(&self) -> &Self::Target {
        &self.model_observation_context
    }
}


impl DerefMut for ChatModelObservationContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
         &mut self.model_observation_context
    }
}
