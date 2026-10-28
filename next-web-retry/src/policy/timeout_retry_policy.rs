//! A policy that retries until a deadline passes.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use next_web_core::Named;

use crate::{
    context::retry_context_support::RetryContextSupport, error::retry_error::RetryError,
    impl_retry_context, retry_context::RetryContext, retry_policy::RetryPolicy,
};

/// The default timeout, in milliseconds.
pub const DEFAULT_TIMEOUT: u64 = 1_000;

/// A policy that retries until a deadline passes.
///
/// The clock starts when the context is created, not when the first attempt starts, so time
/// spent inside a callback counts against the budget. The error type is not inspected: any
/// error is retried while the deadline allows it.
#[derive(Clone)]
pub struct TimeoutRetryPolicy {
    timeout: Arc<std::sync::atomic::AtomicU64>,
}

impl TimeoutRetryPolicy {
    /// Creates a policy with the given timeout in milliseconds.
    pub fn new(timeout_millis: u64) -> Self {
        Self {
            timeout: Arc::new(std::sync::atomic::AtomicU64::new(timeout_millis)),
        }
    }

    /// Returns the timeout in milliseconds.
    pub fn get_timeout(&self) -> u64 {
        self.timeout.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Updates the timeout in milliseconds.
    pub fn set_timeout(&self, timeout_millis: u64) {
        self.timeout
            .store(timeout_millis, std::sync::atomic::Ordering::Release);
    }
}

impl Default for TimeoutRetryPolicy {
    fn default() -> Self {
        Self::new(DEFAULT_TIMEOUT)
    }
}

impl RetryPolicy for TimeoutRetryPolicy {
    fn can_retry(&self, context: &dyn RetryContext) -> bool {
        let any: &dyn std::any::Any = context;
        any.downcast_ref::<TimeoutRetryContext>()
            .map(TimeoutRetryContext::is_alive)
            .unwrap_or(false)
    }

    fn open(&self, parent: Option<Arc<dyn RetryContext>>) -> Arc<dyn RetryContext> {
        Arc::new(TimeoutRetryContext {
            context_support: RetryContextSupport::with_parent(parent),
            timeout: Duration::from_millis(self.get_timeout()),
            start: Instant::now(),
        })
    }

    fn close(&self, _context: &dyn RetryContext) {}

    fn register_error(&self, context: &dyn RetryContext, error: &RetryError) {
        let any: &dyn std::any::Any = context;
        if let Some(context) = any.downcast_ref::<TimeoutRetryContext>() {
            context.context_support.register_error(error);
        }
    }
}

impl Named for TimeoutRetryPolicy {
    fn name(&self) -> &str {
        "TimeoutRetryPolicy"
    }
}

#[derive(Clone)]
struct TimeoutRetryContext {
    context_support: RetryContextSupport,
    timeout: Duration,
    start: Instant,
}

impl TimeoutRetryContext {
    fn is_alive(&self) -> bool {
        self.start.elapsed() <= self.timeout
    }
}

impl_retry_context!(TimeoutRetryContext);

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::TimeoutRetryPolicy;
    use crate::{error::retry_error::RetryError, retry_policy::RetryPolicy};

    #[test]
    fn the_first_attempt_is_allowed_and_a_short_timeout_expires() {
        let policy = TimeoutRetryPolicy::new(1);
        let context = policy.open(None);
        assert!(policy.can_retry(context.as_ref()));

        std::thread::sleep(Duration::from_millis(5));

        assert!(!policy.can_retry(context.as_ref()));
        policy.register_error(context.as_ref(), &RetryError::custom("late"));
        assert_eq!(context.get_retry_count(), 1);
    }

    #[test]
    fn a_long_timeout_keeps_retrying_regardless_of_the_error() {
        let policy = TimeoutRetryPolicy::default();
        let context = policy.open(None);

        policy.register_error(context.as_ref(), &RetryError::custom("fatal"));

        assert!(policy.can_retry(context.as_ref()));
        assert_eq!(policy.get_timeout(), super::DEFAULT_TIMEOUT);
    }
}
