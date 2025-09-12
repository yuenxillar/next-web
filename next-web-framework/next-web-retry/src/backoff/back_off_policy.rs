use std::sync::Arc;

use crate::{
    backoff::back_off_context::BackOffContext,
    error::retry_error::RetryError, retry_context::RetryContext,
};

pub trait BackOffPolicy
where
    Self: Send + Sync,
{
    fn start(&self, context: &dyn RetryContext) -> Option<Arc<dyn BackOffContext>>;

    fn backoff(&self, context: &dyn BackOffContext) -> Result<(), RetryError>;
}
