use next_web_core::DynClone;

pub trait Advisor: DynClone + Send + Sync {}

next_web_core::clone_trait_object!(Advisor);
