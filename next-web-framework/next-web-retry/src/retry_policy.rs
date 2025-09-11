use std::sync::Arc;

use crate::{error::CloneableError, retry_context::RetryContext};

pub trait RetryPolicy
where
    Self: Send + Sync,
{
    // const NO_MAXIMUM_ATTEMPTS_SET: i32 = -1;

    fn can_retry(&self, context: &dyn RetryContext) -> bool;

    // fn open(&self, context:  impl RetryContext) -> Option<Arc<dyn RetryContext>>;

    fn close(&self, context: &dyn RetryContext);

    fn register_error(&mut self, context: &mut dyn RetryContext, error: Option<&dyn CloneableError>);
    fn get_max_attempts(&self) -> i32 {
        return -1;
    }
}
