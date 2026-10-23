//! A marker for back-off policies that pause through a sleeper.

use next_web_core::async_trait;

use crate::{backoff::back_off_policy::BackOffPolicy, error::retry_error::RetryError};

/// Marks a [`BackOffPolicy`] that pauses through a [`Sleeper`](super::sleeper::Sleeper).
///
/// The trait exists so a policy that waits can be identified and its wait can be routed
/// elsewhere, for example to a recording or immediate sleeper in tests and simulations.
#[async_trait]
pub trait SleepingBackOffPolicy
where
    Self: Send + Sync,
    Self: BackOffPolicy,
{
    /// Waits for `back_off_period` milliseconds.
    async fn sleep(&self, back_off_period: u64) -> Result<(), RetryError>;
}
