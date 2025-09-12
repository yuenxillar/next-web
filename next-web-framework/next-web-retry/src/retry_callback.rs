use async_trait::async_trait;

use crate::{error::retry_error::RetryError, retry_context::RetryContext};

#[async_trait]
pub trait RetryCallback<T>
where
    Self: Send + Sync,
{
    async fn do_with_retry(&self, context: &dyn RetryContext) -> Result<T, RetryError>;
}


#[async_trait]
impl<F, Fut, T> RetryCallback<T> for F
where
    F: Send + Sync,
    F: Fn(&dyn RetryContext) -> Fut,
    Fut: Future<Output = Result<T, RetryError>> + Send + Sync,
{
    async fn do_with_retry(&self, context: &dyn RetryContext) -> Result<T, RetryError> {
        self(context).await
    }
}