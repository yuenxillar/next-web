use next_web_core::DynClone;

use crate::{error::AnyError, retry_context::RetryContext};

pub trait RetryPolicy
where
    Self: Send + Sync,
    Self: DynClone
{
    // const NO_MAXIMUM_ATTEMPTS_SET: i32 = -1;

    fn can_retry(&self, context: &dyn RetryContext) -> bool;

    fn open(&self, context:  Option<&dyn RetryContext>) -> Box<dyn RetryContext>;

    fn close(&self, context: &dyn RetryContext);

    fn register_error(&self, context: &mut dyn RetryContext, error: Option<&dyn AnyError>);

    fn get_max_attempts(&self) -> u16 {
        return 0;
    }
}

next_web_core::clone_trait_object!(RetryPolicy);