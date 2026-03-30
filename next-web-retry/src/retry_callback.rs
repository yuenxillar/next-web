use std::{future::Future, marker::PhantomData, pin::Pin};

use next_web_core::async_trait;

use crate::{error::retry_error::RetryError, retry_context::RetryContext};

pub type BoxRetryFuture<'a, T, R = RetryError> =
    Pin<Box<dyn Future<Output = Result<T, R>> + Send + 'a>>;

#[async_trait]
pub trait RetryCallback<T>
where
    Self: Send + Sync,
{
    async fn do_with_retry(&self, context: &dyn RetryContext) -> Result<T, RetryError>;
}

pub struct FnRetryCallback<F, T, R = RetryError> {
    callback: F,
    marker: PhantomData<fn() -> (T, R)>,
}

impl<F, T, R> FnRetryCallback<F, T, R> {
    pub fn new(callback: F) -> Self {
        Self {
            callback,
            marker: PhantomData,
        }
    }
}

pub fn with_fn<F, T, R>(callback: F) -> FnRetryCallback<F, T, R>
where
    F: for<'a> Fn(&'a dyn RetryContext) -> BoxRetryFuture<'a, T, R> + Send + Sync,
    R: Into<RetryError>,
{
    FnRetryCallback::new(callback)
}


#[async_trait]
impl<F, R, T> RetryCallback<T> for FnRetryCallback<F, T, R>
where
    F: for<'a> Fn(&'a dyn RetryContext) -> BoxRetryFuture<'a, T, R> + Send + Sync,
    R: Into<RetryError>,
{
    async fn do_with_retry(&self, context: &dyn RetryContext) -> Result<T, RetryError> {
        (self.callback)(context).await.map_err(Into::into)
    }
}
