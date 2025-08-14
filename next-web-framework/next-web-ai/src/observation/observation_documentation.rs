use next_web_core::error::BoxError;

use crate::{
    chat::observation::observation_convention::ObservationConvention, util::key_name::KeyName,
};

use super::{
    observation::{Context, Observation, ObservationImpl},
    observation_registry::ObservationRegistry,
};

// use crate::chat::observation::observation_convention::ObservationConvention;

pub type BoxObservationConvention = Box<dyn ObservationConvention<Box<dyn Context>>>;

pub trait ObservationDocumentation: Send + Sync {
    fn name(&self) -> Option<&str> {
        None
    }

    fn contextual_name(&self) -> Option<&str> {
        None
    }

    fn default_convention(&self) -> &'static str;

    fn observation(
        &self,
        custom_convention: Option<BoxObservationConvention>,
        default_convention: Option<BoxObservationConvention>,
        context: Box<dyn Context>,
        registry: Box<dyn ObservationRegistry>,
    ) -> Result<Box<dyn Observation>, BoxError> {
        if self.default_convention().is_empty() {
            return Err("default_convention is empty".into());
        }

        if let None = default_convention {
            return Err("default_convention is None".into());
        }

        let mut observation = ObservationImpl::create_not_started(
            custom_convention,
            default_convention,
            context,
            Some(registry),
        );

        if let Some(name) = self.name() {
            observation.context().set_name(name);
        }

        if let Some(contextual_name) = self.contextual_name() {
            observation.contextual_name(contextual_name);
        }

        return Ok(observation);
    }

    fn low_cardinality_key_names(&self) -> Vec<KeyName> {
        vec![]
    }

    fn high_cardinality_key_names(&self) -> Vec<KeyName> {
        vec![]
    }

    fn prefix(&self) -> &str {
        ""
    }

    fn events(&self) -> Vec<i32> {
        vec![]
    }
}
