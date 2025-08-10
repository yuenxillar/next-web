use next_web_core::DynClone;

use crate::chat::observation::chat_model_observation_context::ChatModelObservationContext;

pub trait ChatModelObservationConvention
where
    Self: DynClone,
    Self: Send + Sync,
{
    fn name(&self) -> &str;

    fn contextual_name(&self, context: &ChatModelObservationContext) -> Option<String>;
}

next_web_core::clone_trait_object!(ChatModelObservationConvention);
