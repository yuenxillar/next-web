use std::sync::Arc;

use next_web_core::{async_trait, anys::any_error::AnyError, DynClone};

use crate::retry_context::RetryContext;


pub const NO_MAXIMUM_ATTEMPTS_SET: u16 = 0;

#[async_trait]
pub trait RetryPolicy
where
    Self: Send + Sync,
    Self: DynClone + ToString,
{
    async fn can_retry(&self, context: &dyn RetryContext) -> bool;

    fn open(&self, context:  Option<&dyn RetryContext>) -> Arc<dyn RetryContext>;

    fn close(&self, context: &dyn RetryContext);

    fn register_error(&self, context: &dyn RetryContext, error: Option<&dyn AnyError>);

    fn get_max_attempts(&self) -> u16 {
        return 0;
    }
}

next_web_core::clone_trait_object!(RetryPolicy);