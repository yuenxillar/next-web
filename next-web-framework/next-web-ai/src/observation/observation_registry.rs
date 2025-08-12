use anyhow::Ok;
use next_web_core::DynClone;

use crate::{
    chat::observation::observation_convention::ObservationConvention,
    observation::{noop_observation_registry::NoopObservationRegistry, simple_observation_registry::SimpleObservationRegistry},
    util::sync_array::SyncArray,
};

use super::{
    observation::Context, observation_documentation::BoxObservationConvention,
    observation_filter::ObservationFilter, observation_handler::ObservationHandler,
    observation_predicate::ObservationPredicate,
};

pub trait ObservationRegistry: DynClone + Send + Sync {
    fn current_observation(&self);

    fn current_observation_scope(&self);

    fn set_current_observation_scope(&self);

    fn observation_config(&self) -> &ObservationConfig;

    fn is_noop(&self) -> bool { true }
}

next_web_core::clone_trait_object!(ObservationRegistry);

pub struct ObservationRegistryImpl;

impl ObservationRegistryImpl {
    pub fn create() -> impl ObservationRegistry {
        SimpleObservationRegistry::new()
    }

    pub fn noop() -> impl ObservationRegistry {
        NoopObservationRegistry::new()
    }
}

#[derive(Clone)]
pub struct ObservationConfig {
    pub(crate) observation_handlers: SyncArray<Box<dyn ObservationHandler>>,
    pub(crate) observation_predicates: SyncArray<Box<dyn ObservationPredicate>>,
    pub(crate) observation_conventions: SyncArray<Box<dyn ObservationConvention<Box<dyn Context>>>>,
    pub(crate) observation_filters: SyncArray<Box<dyn ObservationFilter>>,
}

impl ObservationConfig {
    pub fn observation_convention(
        &self,
        context: &dyn Context,
        default_convention: Option<BoxObservationConvention>,
    ) -> anyhow::Result<BoxObservationConvention> {
        for item in self.observation_conventions.data.blocking_read().iter() {
            if item.as_ref().supports_context(context) {
                return Ok(item.clone());
            }
        }
        anyhow::ensure!(
            default_convention.is_none(),
            "No observation convention found for context"
        );
        return Ok(default_convention.unwrap());
    }

    pub fn is_observation_enabled(&self, name: &str, context: &dyn Context) -> bool {
        for predicate in self.observation_predicates.data.blocking_read().iter() {
            if !predicate.test(name, context) {
                return false;
            }
        }
        return true;
    }
}

impl ObservationConfig {
    pub fn with_observation_handlers<R>(
        &self,
        f: impl FnOnce(&[Box<dyn ObservationHandler>]) -> R,
    ) -> R {
        let guard = self.observation_handlers.data.blocking_read();
        f(&guard)
    }

    pub fn with_observation_predicates<R>(
        &self,
        f: impl FnOnce(&[Box<dyn ObservationPredicate>]) -> R,
    ) -> R {
        let guard = self.observation_predicates.data.blocking_read();
        f(&guard)
    }

    pub fn with_observation_conventions<R>(
        &self,
        f: impl FnOnce(&[Box<dyn ObservationConvention<Box<dyn Context>>>]) -> R,
    ) -> R {
        let guard = self.observation_conventions.data.blocking_read();
        f(&guard)
    }

    pub fn with_observation_filters<R>(
        &self,
        f: impl FnOnce(&[Box<dyn ObservationFilter>]) -> R,
    ) -> R {
        let guard = self.observation_filters.data.blocking_read();
        f(&guard)
    }

    pub fn add_observation_handler(&mut self, handler: Box<dyn ObservationHandler>) -> &mut Self {
        self.observation_handlers
            .data
            .blocking_write()
            .push(handler);
        self
    }

    pub fn add_observation_predicate(
        &mut self,
        predicate: Box<dyn ObservationPredicate>,
    ) -> &mut Self {
        self.observation_predicates
            .data
            .blocking_write()
            .push(predicate);
        self
    }

    pub fn add_observation_convention(
        &mut self,
        convention: BoxObservationConvention,
    ) -> &mut Self {
        self.observation_conventions
            .data
            .blocking_write()
            .push(convention);
        self
    }
    pub fn add_observation_filter(&mut self, filter: Box<dyn ObservationFilter>) -> &mut Self {
        self.observation_filters.data.blocking_write().push(filter);
        self
    }
}
