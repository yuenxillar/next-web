use std::any::Any;

use next_web_core::{util::any_map::AnyValue, DynClone};

use crate::error::AnyError;

pub mod retry_context_constants {
    pub const NAME: &str = "context.name";
    pub const STATE_KEY: & str = "context.state";
    pub const CLOSED: &str = "context.closed";
    pub const RECOVERED: &str = "context.recovered";
    pub const EXHAUSTED: &str = "context.exhausted";
    pub const NO_RECOVERY: &str = "context.no-recovery";
    pub const MAX_ATTEMPTS: &str = "context.max-attempts";
}
pub trait RetryContext
where
    Self: Send + Sync,
    Self: Any,
    Self: AttributeAccessor + DynClone,
{
    fn set_exhausted_only(&mut self);

    fn is_exhausted_only(&self) -> bool;

    fn get_parent(&self) -> Option<&dyn RetryContext>;

    fn get_retry_count(&self) -> u16;

    fn get_last_error(&self) -> Option<Box<dyn AnyError>>;
}

next_web_core::clone_trait_object!(RetryContext);

pub trait AttributeAccessor
where
    Self: Send + Sync,
{
    fn has_attribute(&self, name: &str) -> bool;

    fn set_attribute(&mut self, name: &str, value: AnyValue);

    fn remove_attribute(&mut self, name: &str) -> Option<AnyValue>;

    fn get_attribute(&self, name: &str) -> Option<& AnyValue>;
}
