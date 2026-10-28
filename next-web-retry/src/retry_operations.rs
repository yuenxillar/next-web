//! The operations a retry implementation exposes.

use next_web_core::async_trait;

use crate::{
    error::retry_error::RetryError, recovery_callback::RecoveryCallback,
    retry_callback::RetryCallback, retry_state::RetryState,
};

/// Executes an operation with configurable retry behaviour.
///
/// The four methods are the four combinations an application needs: with or without a
/// recovery callback, and stateless or stateful. A stateful execution requires a
/// [`RetryState`] because it recognises the item being processed, which is what lets it retry
/// across calls.
#[async_trait]
pub trait RetryOperations<T>
where
    Self: Send + Sync,
{
    /// Executes the callback until it succeeds or the policy gives up.
    async fn execute(&self, retry_callback: impl RetryCallback<T>) -> Result<T, RetryError>;

    /// Executes the callback, falling back to the recovery callback when the retry is
    /// exhausted.
    async fn execute_with_recovery(
        &self,
        retry_callback: impl RetryCallback<T>,
        recovery_callback: &dyn RecoveryCallback<T>,
    ) -> Result<T, RetryError>;

    /// Executes the callback once per invocation, remembering the item identified by `state`.
    async fn execute_with_state(
        &self,
        retry_callback: impl RetryCallback<T>,
        state: &dyn RetryState,
    ) -> Result<T, RetryError>;

    /// Executes the callback once per invocation, falling back to the recovery callback when
    /// the retry is exhausted.
    async fn execute_with_all(
        &self,
        retry_callback: impl RetryCallback<T>,
        recovery_callback: &dyn RecoveryCallback<T>,
        state: &dyn RetryState,
    ) -> Result<T, RetryError>;
}
