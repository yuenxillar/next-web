use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    backoff::back_off_context::BackOffContext,
    error::retry_error::RetryError, retry_context::RetryContext,
};

#[async_trait]
pub trait BackOffPolicy
where
    Self: Send + Sync,
{
    async fn start(&self, context: &dyn RetryContext) -> Option<Arc<dyn BackOffContext>>;

    async fn backoff(&self, context: Option<&dyn BackOffContext>) -> Result<(), RetryError>;
}
