use std::collections::VecDeque;

use next_web_core::{autoconfigure::context, convert::into_box::IntoBox};

use crate::{
    chat::observation::observation_convention::ObservationConvention,
    observation::observation_documentation::BoxObservationConvention,
};

use super::{
    observation::{Context, Observation},
    observation_filter::ObservationFilter,
    observation_handler::ObservationHandler,
    observation_registry::ObservationRegistry,
};


pub struct SimpleObservation {
    pub(crate) context: Box<dyn Context>,
    pub(crate) registry: Box<dyn ObservationRegistry>,
    pub(crate) convention: Option<BoxObservationConvention>,
    pub(crate) handlers: VecDeque<Box<dyn ObservationHandler>>,
    pub(crate) filters: Vec<Box<dyn ObservationFilter>>,
}

impl SimpleObservation {
    pub fn new(
        name: impl AsRef<str>,
        registry: Box<dyn ObservationRegistry>,
        mut context: impl Context + 'static,
    ) -> Self {
        context.set_name(name.as_ref());
        let filters = registry
            .observation_config()
            .observation_filters
            .data
            .blocking_read()
            .clone();
        Self {
            convention: Self::get_convention_from_config(&registry, &context),
            handlers: Self::get_handlers_from_config(&registry, &context),
            context: Box::new(context),
            filters,
            registry,
        }
    }

    fn get_convention_from_config(
        registry: &Box<dyn ObservationRegistry>,
        context: &dyn Context,
    ) -> Option<BoxObservationConvention> {
        for convention in registry
            .observation_config()
            .observation_conventions
            .data
            .blocking_read()
            .iter()
        {
            if convention.supports_context(context) {
                return Some(convention.clone());
            }
        }
        return None;
    }

    fn get_handlers_from_config(
        registry: &Box<dyn ObservationRegistry>,
        context: &dyn Context,
    ) -> VecDeque<Box<dyn ObservationHandler>> {
        let mut handlers = VecDeque::new();
        for handler in registry
            .observation_config()
            .observation_handlers
            .data
            .blocking_read()
            .iter()
        {
            if handler.supports_context(context) {
                handlers.push_back(handler.clone());
            }
        }
        handlers
    }
}

impl SimpleObservation {
    fn notify_on_observation_started(&mut self) {
        for handler in self.handlers.iter_mut() {
            handler.on_start(self.context.as_ref());
        }
    }

    fn notify_on_observation_stopped(&mut self, context: &dyn Context) {
        self.handlers
            .iter_mut()
            .rev()
            .for_each(|handler| handler.on_stop(context));
    }
}
impl Observation for SimpleObservation {
    fn start(&mut self) {
        if let Some(convention) = self.convention.as_ref() {
            let low = convention.low_cardinality_key_values(self.context.as_ref());
            let high = convention.high_cardinality_key_values(self.context.as_ref());

            self.context.add_low_cardinality_key_values(low);
            self.context.add_low_cardinality_key_values(high);

            let new_name = convention.name();
            if let Some(name) = new_name {
                self.context.set_name(name);
            }
        }
        self.notify_on_observation_started();
    }

    fn stop(&mut self) {
        if let Some(convention) = self.convention.as_ref() {
            let low = convention.low_cardinality_key_values(self.context.as_ref());
            let high = convention.high_cardinality_key_values(self.context.as_ref());

            self.context.add_low_cardinality_key_values(low);
            self.context.add_low_cardinality_key_values(high);

            let new_name = convention.contextual_name(self.context.as_ref());
            if let Some(name) = new_name {
                self.context.set_contextual_name(name);
            }
        }

        let mut modified_context = self.context.clone();
        for filter in self.filters.iter_mut() {
            modified_context = filter.map(modified_context);
        }

        self.notify_on_observation_stopped(modified_context.as_ref());
    }

    fn context(&mut self) -> &mut dyn super::observation::Context {
        self.context.as_mut()
    }

    fn contextual_name(&mut self, contextual_name: &str) {
        self.context.set_contextual_name(contextual_name);
    }
    
    fn parent_observation(&mut self, parent_observation: Box<dyn Observation>) {
        todo!()
    }
    
    fn low_cardinality_key_value(&mut self, key_value: Box<dyn crate::util::key_value::KeyValue>) {
        todo!()
    }
    
    fn high_cardinality_key_value(&mut self, key_value: Box<dyn crate::util::key_value::KeyValue>) {
        todo!()
    }
    
    fn observation_convention(&mut self, observation_convention: BoxObservationConvention) {
        todo!()
    }
    
    fn error(&mut self, error: next_web_core::error::BoxError) {
        todo!()
    }
    
    fn event(&mut self, event: Box<dyn super::observation::Event>) {
        todo!()
    }
    
    fn open_scope(&self) -> Box<dyn super::observation::Scope> {
        todo!()
    }
}
