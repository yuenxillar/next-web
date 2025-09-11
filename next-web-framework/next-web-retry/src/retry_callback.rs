use std::error::Error;

use async_trait::async_trait;

use crate::retry_context::RetryContext;


#[async_trait]
pub trait RetryCallback<T, E>
where
    Self: Send + Sync,
    E: Error,
{
    async fn do_with_retry(&self, context: &dyn RetryContext) -> Result<T, E>;
}


#[async_trait]
impl<F, Fut, T, E> RetryCallback<T, E> for F
where
    E: Error,
    F: Send + Sync,
    F: Fn(&dyn RetryContext) -> Fut,
    Fut: Future<Output = Result<T, E>> + Send + Sync,
{
    async fn do_with_retry(&self, context: &dyn RetryContext) -> Result<T, E> {
        self(context).await
    }
}
