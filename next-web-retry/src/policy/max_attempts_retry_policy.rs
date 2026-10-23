//! A policy that retries a fixed number of times.

use std::sync::Arc;

use next_web_core::Named;

use crate::{
    context::retry_context_support::RetryContextSupport, error::retry_error::RetryError,
    retry_context::RetryContext, retry_policy::RetryPolicy,
};

/// The default number of attempts of a new policy.
pub const DEFAULT_MAX_ATTEMPTS: u16 = 3;

/// A policy that retries a fixed number of times and ignores the error type.
///
/// The attempt count includes the initial call, so a policy built with `3` allows one
/// initial attempt and two retries.
///
/// Exception classification is generally required in production: retrying an
/// unrecoverable failure such as an out-of-memory condition only delays it. Compose this
/// policy with [`BinaryErrorClassifierRetryPolicy`](super::binary_error_classifier_retry_policy::BinaryErrorClassifierRetryPolicy)
/// through [`CompositeRetryPolicy`](super::composite_retry_policy::CompositeRetryPolicy),
/// which is exactly what [`RetryTemplate::builder`](crate::support::retry_template::RetryTemplate::builder)
/// does.
#[derive(Clone)]
pub struct MaxAttemptsRetryPolicy {
    max_attempts: Arc<std::sync::atomic::AtomicU16>,
}

impl MaxAttemptsRetryPolicy {
    /// Creates a policy with the given number of attempts.
    pub fn new(max_attempts: u16) -> Self {
        Self {
            max_attempts: Arc::new(std::sync::atomic::AtomicU16::new(max_attempts)),
        }
    }

    /// Updates the number of attempts.
    ///
    /// The value is shared, so a change made while another task is retrying affects its
    /// next `can_retry` call, which is what makes the limit adjustable at runtime.
    pub fn set_max_attempts(&self, max_attempts: u16) {
        self.max_attempts
            .store(max_attempts, std::sync::atomic::Ordering::Release);
    }
}

impl Default for MaxAttemptsRetryPolicy {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_ATTEMPTS)
    }
}

impl RetryPolicy for MaxAttemptsRetryPolicy {
    fn can_retry(&self, context: &dyn RetryContext) -> bool {
        context.get_retry_count() < self.get_max_attempts()
    }

    fn open(&self, parent: Option<Arc<dyn RetryContext>>) -> Arc<dyn RetryContext> {
        Arc::new(RetryContextSupport::with_parent(parent))
    }

    fn close(&self, _context: &dyn RetryContext) {}

    fn register_error(&self, context: &dyn RetryContext, error: &RetryError) {
        let any: &dyn std::any::Any = context;
        if let Some(context) = any.downcast_ref::<RetryContextSupport>() {
            context.register_error(error);
        }
    }

    fn get_max_attempts(&self) -> u16 {
        self.max_attempts.load(std::sync::atomic::Ordering::Acquire)
    }
}

impl Named for MaxAttemptsRetryPolicy {
    fn name(&self) -> &str {
        "MaxAttemptsRetryPolicy"
    }
}

#[cfg(test)]
mod tests {
    use super::MaxAttemptsRetryPolicy;
    use crate::{error::retry_error::RetryError, retry_policy::RetryPolicy};

    #[test]
    fn retries_until_the_limit_is_reached() {
        let policy = MaxAttemptsRetryPolicy::new(2);
        let context = policy.open(None);

        assert!(policy.can_retry(context.as_ref()));
        policy.register_error(context.as_ref(), &RetryError::custom("first"));
        assert!(policy.can_retry(context.as_ref()));
        policy.register_error(context.as_ref(), &RetryError::custom("second"));
        assert!(!policy.can_retry(context.as_ref()));
        assert_eq!(policy.get_max_attempts(), 2);
    }

    #[test]
    fn the_limit_can_change_while_the_context_is_alive() {
        let policy = MaxAttemptsRetryPolicy::new(1);
        let context = policy.open(None);
        policy.register_error(context.as_ref(), &RetryError::custom("first"));

        assert!(!policy.can_retry(context.as_ref()));
        policy.set_max_attempts(3);
        assert!(policy.can_retry(context.as_ref()));
    }
}
