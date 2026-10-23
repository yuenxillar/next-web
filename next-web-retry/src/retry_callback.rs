//! The callback a retry block executes, and the closure adapter for it.

use std::marker::PhantomData;

use next_web_core::async_trait;

use crate::{error::retry_error::RetryError, retry_context::RetryContext};

/// The operation a retry block re-executes until it succeeds or the policy gives up.
///
/// A callback receives the current context, so it can read the attempt count or store its
/// own bookkeeping between attempts.
#[async_trait]
pub trait RetryCallback<T>
where
    Self: Send + Sync,
{
    /// Runs one attempt.
    async fn do_with_retry(&self, context: &dyn RetryContext) -> Result<T, RetryError>;

    /// Returns a stable label used for reporting.
    ///
    /// The label ends up in the [`crate::retry_context::retry_context_constants::NAME`]
    /// attribute, which is what listeners and statistics use to group retries.
    fn label(&self) -> Option<&str> {
        None
    }
}

/// Adapts a closure to [`RetryCallback`].
///
/// The closure must return a boxed future because [`RetryCallback`] is used through
/// `impl` bounds, and a plain `async` closure cannot name the lifetime of the borrowed
/// context in a stable way. Use [`with_fn`] rather than building this type directly.
pub struct FnRetryCallback<F, T, R = RetryError> {
    callback: F,
    marker: PhantomData<fn() -> (T, R)>,
}

impl<F, T, R> FnRetryCallback<F, T, R> {
    /// Wraps a closure.
    pub fn new(callback: F) -> Self {
        Self {
            callback,
            marker: PhantomData,
        }
    }
}

/// Creates a [`RetryCallback`] from a closure.
///
/// The closure receives the retry context and returns a boxed future that resolves to
/// `Result<T, R>`. Any error type that converts into [`RetryError`] is accepted.
///
/// # Example
///
/// ```
/// use next_web_retry::{
///     error::retry_error::RetryError,
///     retry_callback::with_fn,
///     support::retry_template::RetryTemplate,
///     retry_operations::RetryOperations,
/// };
///
/// # async fn example() {
/// let result = RetryTemplate::builder()
///     .max_attempts(2)
///     .build()
///     .execute(with_fn(|_context| {
///         Box::pin(async { Ok::<_, RetryError>(41 + 1) })
///     }))
///     .await;
///
/// assert_eq!(result.expect("attempt should succeed"), 42);
/// # }
/// ```
pub fn with_fn<F, T, R>(callback: F) -> FnRetryCallback<F, T, R>
where
    F: for<'a> Fn(
            &'a dyn RetryContext,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, R>> + Send + 'a>>
        + Send
        + Sync,
    R: Into<RetryError> + Send + 'static,
    T: Send + 'static,
{
    FnRetryCallback::new(callback)
}

#[async_trait]
impl<F, R, T> RetryCallback<T> for FnRetryCallback<F, T, R>
where
    F: for<'a> Fn(
            &'a dyn RetryContext,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, R>> + Send + 'a>>
        + Send
        + Sync,
    R: Into<RetryError> + Send + 'static,
    T: Send + 'static,
{
    async fn do_with_retry(&self, context: &dyn RetryContext) -> Result<T, RetryError> {
        (self.callback)(context).await.map_err(Into::into)
    }
}
