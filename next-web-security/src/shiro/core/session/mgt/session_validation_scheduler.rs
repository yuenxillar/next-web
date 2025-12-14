use next_web_core::{clone_trait_object, DynClone};

pub trait SessionValidationScheduler
where
    Self: Send + Sync,
    Self: DynClone,
{
    fn is_enabled(&self) -> bool;

    fn enable_session_validation(&self);

    fn disable_session_validation(&mut self);
}

clone_trait_object!(SessionValidationScheduler);
