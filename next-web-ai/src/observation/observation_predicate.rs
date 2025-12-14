use next_web_core::DynClone;

use super::observation::Context;

pub trait ObservationPredicate: DynClone + Send + Sync {
    fn test(&self, t: &str, u: &dyn Context) -> bool;
}

next_web_core::clone_trait_object!(ObservationPredicate);
