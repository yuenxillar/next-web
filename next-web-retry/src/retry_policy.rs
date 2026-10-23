//! Retry policies decide whether another attempt is allowed.

use std::sync::Arc;

use next_web_core::Named;

use crate::{error::retry_error::RetryError, retry_context::RetryContext};

/// The value [`RetryPolicy::get_max_attempts`] returns when a policy has no fixed limit.
pub const NO_MAXIMUM_ATTEMPTS_SET: u16 = 0;

/// Decides whether a retry block may make another attempt.
///
/// A policy is the heart of a retry block: it owns the per-attempt state through the
/// [`RetryContext`] it creates in [`RetryPolicy::open`], it counts failures in
/// [`RetryPolicy::register_error`], and it answers [`RetryPolicy::can_retry`] after every
/// failure.
///
/// Implementations must be safe to share between tasks. Policies are usually stateless
/// themselves; the per-attempt state lives in the context.
pub trait RetryPolicy
where
    Self: Send + Sync,
    Self: Named,
{
    /// Returns whether another attempt is allowed for this context.
    fn can_retry(&self, context: &dyn RetryContext) -> bool;

    /// Creates the context for a new retry block.
    ///
    /// `parent` is the context of the enclosing retry block when this one is nested, and
    /// `None` otherwise.
    fn open(&self, parent: Option<Arc<dyn RetryContext>>) -> Arc<dyn RetryContext>;

    /// Releases whatever the policy acquired in [`RetryPolicy::open`].
    fn close(&self, context: &dyn RetryContext);

    /// Records a failed attempt.
    ///
    /// This must be called exactly once per failed attempt, because the default
    /// implementations increment the attempt counter here.
    fn register_error(&self, context: &dyn RetryContext, error: &RetryError);

    /// Returns the maximum number of attempts, or [`NO_MAXIMUM_ATTEMPTS_SET`].
    fn get_max_attempts(&self) -> u16 {
        NO_MAXIMUM_ATTEMPTS_SET
    }
}

#[cfg(test)]
mod tests {
    use super::NO_MAXIMUM_ATTEMPTS_SET;

    #[test]
    fn the_sentinel_value_marks_an_unbounded_policy() {
        assert_eq!(NO_MAXIMUM_ATTEMPTS_SET, 0);
    }
}
