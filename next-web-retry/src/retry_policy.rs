use std::sync::Arc;

use next_web_core::{anys::any_error::AnyError, async_trait, traits::named::Named};

use crate::retry_context::RetryContext;

pub const NO_MAXIMUM_ATTEMPTS_SET: u16 = 0;

#[async_trait]
pub trait RetryPolicy
where
    Self: Send + Sync,
    Self: Named,
{
    async fn can_retry(&self, context: &dyn RetryContext) -> bool;

    fn open(&self, context: Option<&dyn RetryContext>) -> Arc<dyn RetryContext>;

    fn close(&self, context: &dyn RetryContext);

    fn register_error(&self, context: &dyn RetryContext, error: Option<&dyn AnyError>);

    fn get_max_attempts(&self) -> u16 {
        u16::MIN
    }
}
