use next_web_core::{autoconfigure::context, DynClone};

use crate::observation::observation::{Context, Event};

pub trait ObservationHandler: DynClone + Send + Sync {

    fn on_start(&mut self, context: &dyn Context);

    fn on_stop(&mut self, context: &dyn Context);

    fn on_error(&mut self, context: &dyn Context);

    fn on_event(&mut self, event: &dyn Event, context: &dyn Context);

    fn on_scope_closed(&mut self, context: &dyn Context);

    fn on_scope_reset(&mut self, context: &dyn Context);

    fn supports_context(&self, context: &dyn Context) -> bool;
}

next_web_core::clone_trait_object!(ObservationHandler);
