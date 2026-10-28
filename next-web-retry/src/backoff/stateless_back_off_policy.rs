//! A back off that keeps no state between waits.

use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    backoff::{back_off_context::BackOffContext, back_off_policy::BackOffPolicy},
    error::retry_error::RetryError,
    retry_context::RetryContext,
};

/// A [`BackOffPolicy`] that keeps no state between waits.
///
/// The template still calls [`BackOffPolicy::start`], so a stateless policy returns `None`
/// there and does all of its work in [`StatelessBackOffPolicy::do_backoff`]. Implementations
/// get the default `start` and `backoff` behaviour for free.
#[async_trait]
pub trait StatelessBackOffPolicy
where
    Self: Send + Sync,
    Self: BackOffPolicy,
{
    /// Performs the wait. The back-off context is not needed because the policy is
    /// stateless.
    async fn do_backoff(&self) -> Result<(), RetryError>;
}

/// A helper that gives a [`StatelessBackOffPolicy`] the standard [`BackOffPolicy`] behaviour.
///
/// Rust cannot provide a default implementation of a supertrait method from a subtrait, so
/// stateless policies call this from their `start` and `backoff` implementations instead of
/// repeating the two one-line bodies.
pub async fn stateless_start(_context: &dyn RetryContext) -> Option<Arc<dyn BackOffContext>> {
    None
}

/// Runs [`StatelessBackOffPolicy::do_backoff`] from a [`BackOffPolicy::backoff`]
/// implementation.
pub async fn stateless_backoff<T>(policy: &T) -> Result<(), RetryError>
where
    T: StatelessBackOffPolicy + ?Sized,
{
    policy.do_backoff().await
}
