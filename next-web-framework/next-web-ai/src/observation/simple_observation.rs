use std::collections::VecDeque;

use crate::chat::observation::observation_convention::ObservationConvention;

use super::{
    observation::{Context, Observation},
    observation_filter::ObservationFilter,
    observation_handler::ObservationHandler,
    observation_registry::ObservationRegistry,
};

pub struct SimpleObservation {
    pub(crate) context: Box<dyn Context>,
    pub(crate) registry: Box<dyn ObservationRegistry>,
    pub(crate) convention: Box<dyn ObservationConvention<Box<dyn Context>>>,
    pub(crate) handlers: VecDeque<Box<dyn ObservationHandler>>,
    pub(crate) filters: Vec<Box<dyn ObservationFilter>>,
}

impl SimpleObservation {}

impl Observation for SimpleObservation {
    fn start(&self) {
        todo!()
    }

    fn context(&mut self) -> &mut dyn super::observation::Context {
        todo!()
    }

    fn stop(&self) {
        todo!()
    }

    fn contextual_name(&self, contextual_name: &str) {
        todo!()
    }
}
