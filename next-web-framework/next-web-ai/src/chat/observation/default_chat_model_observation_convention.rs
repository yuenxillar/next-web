use crate::{
    chat::observation::{
        chat_model_observation_context::ChatModelObservationContext,
        chat_model_observation_convention::ChatModelObservationConvention, observation_convention::ObservationConvention,
    },
    util::key_value::{KeyValue, NoneKeyValue, NONE_VALUE},
};

use super::chat_model_observation_documentation::{ChatModelObservationDocumentation, LowNames};

const DEFAULT_NAME: &str = "gen_ai.client.operation";

#[derive(Clone)]
pub struct DefaultChatModelObservationConvention {
    request_model_none: Box<dyn KeyValue>,
    response_model_name: Box<dyn KeyValue>,
}

impl ChatModelObservationConvention for DefaultChatModelObservationConvention {
    fn name(&self) -> &str {
        DEFAULT_NAME
    }

    fn contextual_name(&self, context: &ChatModelObservationContext) -> Option<String> {
        let model = context.request().chat_options().get_model();
        if model.is_empty() {
            return Some(format!("{:?} {}", context.operation_metadata(), model));
        }
        return Some(context.operation_metadata().operation_type.clone());
    }
}


impl<T> ObservationConvention<T> for DefaultChatModelObservationConvention {
    fn supports_context(&self, context: &dyn crate::observation::observation::Context) -> bool {
        true
    }
}

impl Default for DefaultChatModelObservationConvention {
    fn default() -> Self {
        Self {
            request_model_none: Box::new(NoneKeyValue::of_immutable(
                ChatModelObservationDocumentation::LowCardinalityKeyNames(LowNames::RequestModel)
                    .value(),
                NONE_VALUE,
            )),
            response_model_name: Box::new(NoneKeyValue::of_immutable(
                ChatModelObservationDocumentation::LowCardinalityKeyNames(LowNames::ResponseModel)
                    .value(),
                NONE_VALUE,
            )),
        }
    }
}
