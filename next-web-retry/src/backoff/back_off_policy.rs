//! The strategy that decides how long to wait before the next attempt.

use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    backoff::back_off_context::BackOffContext, error::retry_error::RetryError,
    retry_context::RetryContext,
};

/// Controls the pause between two attempts of one retry block.
///
/// For every block the template calls [`BackOffPolicy::start`] once, and
/// [`BackOffPolicy::backoff`] before each retry. A policy that needs to remember where it
/// is in its schedule returns a [`BackOffContext`] from `start`; a stateless policy returns
/// `None`.
///
/// Implementations are expected to be thread-safe and shareable: one template is used by
/// many tasks at once, and each task has its own back-off context.
#[async_trait]
pub trait BackOffPolicy
where
    Self: Send + Sync,
{
    /// Starts a new block of waits and returns its state, if the policy keeps any.
    async fn start(&self, context: &dyn RetryContext) -> Option<Arc<dyn BackOffContext>>;

    /// Waits before the next attempt.
    ///
    /// Returns [`RetryError::BackOffInterruptedError`] when the wait was prevented from
    /// outside; the template then aborts the whole retry instead of trying again.
    async fn backoff(&self, context: Option<&dyn BackOffContext>) -> Result<(), RetryError>;
}
