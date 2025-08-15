use std::collections::VecDeque;

use next_web_core::{async_trait, convert::into_box::IntoBox, error::BoxError, DynClone};

use crate::{
    chat::observation::observation_convention::ObservationConvention,
    observation::{noop_observation::NoopObservation, simple_event::SimpleEvent},
    util::{key_value::KeyValue, key_values::KeyValues},
};

use super::{
    observation_documentation::BoxObservationConvention, observation_registry::ObservationRegistry,
    simple_observation::SimpleObservation,
};

// #[async_trait]
pub trait Observation: Send + Sync {
    fn start(&mut self);

    fn stop(&mut self);

    fn context(&mut self) -> &mut dyn Context;

    fn contextual_name(&mut self, contextual_name: &str);

    fn parent_observation(&mut self, parent_observation: Box<dyn Observation>);

    fn low_cardinality_key_value(&mut self, key_value: Box<dyn KeyValue>);

    fn high_cardinality_key_value(&mut self, key_value: Box<dyn KeyValue>);

    fn observation_convention(&mut self, observation_convention: BoxObservationConvention);

    fn error(&mut self, error: &BoxError);

    fn event(&mut self, event: Box<dyn Event>);

    fn open_scope(&self) -> Box<dyn Scope>;
}

#[async_trait]
pub trait Observable {
    async fn observe<R, 'a>(
        &mut self,
        run: impl std::future::Future<Output = Result<R, BoxError>> + Send + 'a
    ) -> Result<R, BoxError>;
}

#[async_trait]
impl<T: Observation + ?Sized> Observable for T {
    async fn observe<R, 'a>(
        &mut self,
        run: impl std::future::Future<Output = Result<R, BoxError>> + Send + 'a,
    ) -> Result<R, BoxError> {
        self.start();
        match run.await {
            Ok(value) => {
                self.stop();
                Ok(value)
            }
            Err(e) => {
                self.error(&e);
                self.stop();

                Err(e)
            }
        }
    }
}

pub struct ObservationImpl;

impl ObservationImpl {
    pub fn create_not_started(
        custom_convention: Option<BoxObservationConvention>,
        default_convention: Option<BoxObservationConvention>,
        context: impl Context + 'static,
        registry: Option<Box<dyn ObservationRegistry>>,
    ) -> Box<dyn Observation> {
        if registry.is_none() || registry.as_ref().map(|s| s.is_noop()).unwrap_or(false) {
            return Self::noop().into_boxed();
        }

        let registry = registry.unwrap();

        let mut context = Box::new(context);
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
            return Self::noop().into_boxed();
        }

        let convention = Some(convention);
        SimpleObservation {
            context,
            registry,
            convention,
            handlers: VecDeque::new(),
            filters: Vec::new(),
        }
        .into_boxed()
    }

    pub fn start(name: impl Into<String>, registry: Box<dyn ObservationRegistry>) {
        return Self::create_not_started_from_name(name.into(), DefaultContext::new(), registry)
            .start();
    }

    pub fn create_not_started_from_name(
        name: impl Into<String>,
        mut context: impl Context + 'static,
        registry: Box<dyn ObservationRegistry>,
    ) -> Box<dyn Observation> {
        if registry.is_noop() {
            return Self::noop().into_boxed();
        }

        let name = name.into();
        let is_observation_enabled = !registry
            .observation_config()
            .is_observation_enabled(name.as_str(), &context);

        context.set_parent_from_current_observation(registry.as_ref());

        if is_observation_enabled {
            return Self::noop().into_boxed();
        }

        SimpleObservation::new(name, registry, context).into_boxed()
    }
    pub fn noop() -> impl Observation {
        NoopObservation::default()
    }
}

pub trait Context: Send + Sync + DynClone {
    fn set_parent_from_current_observation(&mut self, registry: &dyn ObservationRegistry);

    fn add_low_cardinality_key_values(&mut self, key_values: KeyValues<Box<dyn KeyValue>>);

    fn set_name(&mut self, name: &str);

    fn set_contextual_name(&mut self, contextual_name: &str);
}

next_web_core::clone_trait_object!(Context);

#[derive(Clone)]
pub struct DefaultContext {}

impl DefaultContext {
    pub fn new() -> Self {
        Self {}
    }
}
impl Context for DefaultContext {
    fn set_parent_from_current_observation(&mut self, registry: &dyn ObservationRegistry) {}

    fn set_name(&mut self, name: &str) {}

    fn add_low_cardinality_key_values(&mut self, key_values: KeyValues<Box<dyn KeyValue>>) {}

    fn set_contextual_name(&mut self, contextual_name: &str) {}
}

pub trait Event: Send + Sync {
    fn name(&self) -> &str;

    fn wall_time(&self) -> u64 {
        0
    }

    fn contextual_name(&self) -> &str {
        self.name()
    }
}

pub struct EventImpl;

impl EventImpl {
    pub fn of<T>(name: T, contextual_name: T) -> impl Event
    where
        T: Into<String>,
    {
        SimpleEvent::new(name, contextual_name)
    }

    pub fn of_name<T>(name: T) -> impl Event
    where
        T: Into<String>,
    {
        let name: String = name.into();
        SimpleEvent::new(name.clone(), name)
    }
}

pub trait Scope: Send + Sync {
    fn current_observation(&self) -> Option<&dyn Observation>;

    fn previous_observation_scope(&self) -> Option<&dyn Scope>;

    fn close(&mut self);

    fn reset(&mut self);

    fn make_current(&mut self);

    fn is_noop(&self) -> bool {
        true
    }
}
