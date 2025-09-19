use std::sync::Arc;

use next_web_core::async_trait;

use crate::{error::retry_error::RetryError, retry_context::RetryContext};

#[async_trait]
pub trait RetryCallback<T>
where
    Self: Send+ Sync,
{
    async fn do_with_retry(&self, context: Arc<dyn RetryContext>) -> Result<T, RetryError>;
}

#[async_trait]
impl<F, Fut, T> RetryCallback<T> for F
where
    F: Fn(Arc<dyn RetryContext>) -> Fut + Send + Sync,
    Fut: Future<Output = Result<T, RetryError>> + Send + 'static,
{
    async fn do_with_retry(&self, context: Arc<dyn RetryContext>) -> Result<T, RetryError> {
        self(context).await
    }
}