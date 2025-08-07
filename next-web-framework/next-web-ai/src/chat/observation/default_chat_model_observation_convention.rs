use crate::chat::observation::{
    chat_model_observation_context::ChatModelObservationContext,
    chat_model_observation_convention::ChatModelObservationConvention,
};

const DEFAULT_NAME: &str = "gen_ai.client.operation";

#[derive(Clone)]
pub struct DefaultChatModelObservationConvention {}

impl ChatModelObservationConvention for DefaultChatModelObservationConvention {
    fn get_name(&self) -> &str {
        DEFAULT_NAME
    }

    fn get_contextual_name(&self, context: &ChatModelObservationContext) -> Option<String> {
        if context.request_options().get_model().is_empty() {
            return Some(format!(
                "{:?} {}",
                context.operation_metadata(),
                context.request_options().get_model()
            ));
        }
        return Some(context.operation_metadata().operation_type.clone());
    }
}
