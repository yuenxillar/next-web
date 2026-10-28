//! Recovery callbacks produce the fallback value of a failed retry block.

use next_web_core::async_trait;

use crate::{error::retry_error::RetryError, retry_context::RetryContext};

/// Produces the value of a retry block after every attempt failed.
///
/// A recovery callback is the fallback path of a retry: instead of propagating the last
/// error, the block returns whatever the recoverer produces. It is only invoked when the
/// context does not carry [`crate::retry_context::retry_context_constants::NO_RECOVERY`],
/// which policies set for errors that must be surfaced immediately.
#[async_trait]
pub trait RecoveryCallback<T>
where
    Self: Send + Sync,
{
    /// Produces a replacement result for a failed retry block.
    async fn recover(&self, context: &dyn RetryContext) -> Result<T, RetryError>;
}

/// A recovery callback that always returns a fixed value.
///
/// Useful in tests and for retry blocks whose fallback is a constant.
pub struct ConstantRecoveryCallback<T> {
    value: T,
}

impl<T> ConstantRecoveryCallback<T> {
    /// Creates a callback that returns `value`.
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

#[async_trait]
impl<T> RecoveryCallback<T> for ConstantRecoveryCallback<T>
where
    T: Clone + Send + Sync,
{
    async fn recover(&self, _context: &dyn RetryContext) -> Result<T, RetryError> {
        Ok(self.value.clone())
    }
}
