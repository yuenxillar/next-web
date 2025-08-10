use next_web_core::DynClone;

pub trait ObservationHandler: DynClone + Send + Sync {}

next_web_core::clone_trait_object!(ObservationHandler);
