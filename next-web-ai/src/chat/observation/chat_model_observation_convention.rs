use crate::chat::observation::{
    chat_model_observation_context::ChatModelObservationContext,
    observation_convention::ObservationConvention,
};

pub trait ChatModelObservationConvention
where
    Self: ObservationConvention<ChatModelObservationContext>,
{
    fn name(&self) -> &str;

    fn contextual_name(&self, context: &ChatModelObservationContext) -> Option<String>;
}

next_web_core::clone_trait_object!(ChatModelObservationConvention);
