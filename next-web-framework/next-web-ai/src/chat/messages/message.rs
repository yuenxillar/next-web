use next_web_core::DynClone;

pub trait Message: DynClone + Send + Sync {}

next_web_core::clone_trait_object!(Message);
