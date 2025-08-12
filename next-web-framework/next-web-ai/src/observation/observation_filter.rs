use next_web_core::DynClone;

use crate::observation::observation::Context;

pub trait ObservationFilter: DynClone + Send + Sync {

    fn map(&self, context: Box<dyn Context>) -> Box<dyn Context>;
}

next_web_core::clone_trait_object!(ObservationFilter);
