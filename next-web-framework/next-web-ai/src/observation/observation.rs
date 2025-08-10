use std::collections::VecDeque;

use next_web_core::DynClone;

use crate::chat::observation::observation_convention::ObservationConvention;

use super::{
    observation_documentation::BoxObservationConvention, observation_registry::ObservationRegistry,
    simple_observation::SimpleObservation,
};

pub trait Observation: Send + Sync {
    fn start(&self);

    fn context(&mut self) -> &mut dyn Context;

    fn stop(&self);

    fn contextual_name(&self, contextual_name: &str);
}

pub struct NoopObservation {}

impl Observation for NoopObservation {
    fn start(&self) {
        todo!()
    }

    fn context(&mut self) -> &mut dyn Context {
        todo!()
    }

    fn stop(&self) {
        todo!()
    }

    fn contextual_name(&self, contextual_name: &str) {
        todo!()
    }
}

impl Default for NoopObservation {
    fn default() -> Self {
        Self {}
    }
}

pub struct ObservationImpl;

impl ObservationImpl {
    pub fn create_not_started(
        custom_convention: Option<BoxObservationConvention>,
        default_convention: Option<BoxObservationConvention>,
        mut context: Box<dyn Context>,
        registry: Option<Box<dyn ObservationRegistry>>,
    ) -> Box<dyn Observation> {
        if registry.is_none() || registry.as_ref().map(|s| s.is_noop()).unwrap_or(false) {
            return Box::new(NoopObservation::default());
        }
        let registry = registry.unwrap();

        let convention: Box<dyn ObservationConvention<Box<dyn Context>>>;
        if let Some(custom_convention) = custom_convention {
            convention = custom_convention;
        } else {
            convention = registry
                .observation_config()
                .observation_convention(context.as_ref(), default_convention)
                .unwrap();
        }

        let is_observation_enabled = !registry
            .observation_config()
            .is_observation_enabled(convention.name().unwrap_or_default(), context.as_ref());

        context.set_parent_from_current_observation(registry.as_ref());

        if is_observation_enabled {
            return Box::new(NoopObservation::default());
        }

        Box::new(SimpleObservation {
            context,
            registry,
            convention,
            handlers: VecDeque::new(),
            filters: Vec::new(),
        })
    }
}

pub trait Context: Send + Sync + DynClone {
    fn set_parent_from_current_observation(&mut self, registry: &dyn ObservationRegistry);

    fn set_name(&mut self, name: &str);
}

next_web_core::clone_trait_object!(Context);
