//! A policy that always allows another attempt.

use std::sync::Arc;

use next_web_core::Named;

use crate::{
    context::retry_context_support::RetryContextSupport, error::retry_error::RetryError,
    retry_context::RetryContext, retry_policy::RetryPolicy,
};

/// A policy that always allows another attempt.
///
/// Use it for retries that must continue until the callback succeeds, together with a
/// mechanism that can stop them from outside, such as a timeout in the callback or a
/// listener that marks the context exhausted.
#[derive(Clone, Copy, Default, Debug)]
pub struct AlwaysRetryPolicy;

impl AlwaysRetryPolicy {
    /// Creates the policy.
    pub fn new() -> Self {
        Self
    }
}

impl RetryPolicy for AlwaysRetryPolicy {
    fn can_retry(&self, _context: &dyn RetryContext) -> bool {
        true
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
}

impl Named for AlwaysRetryPolicy {
    fn name(&self) -> &str {
        "AlwaysRetryPolicy"
    }
}

#[cfg(test)]
mod tests {
    use super::AlwaysRetryPolicy;
    use crate::{error::retry_error::RetryError, retry_policy::RetryPolicy};

    #[test]
    fn every_failure_allows_another_attempt() {
        let policy = AlwaysRetryPolicy::new();
        let context = policy.open(None);

        for _ in 0..100 {
            policy.register_error(context.as_ref(), &RetryError::custom("failed"));
        }

        assert!(policy.can_retry(context.as_ref()));
        assert_eq!(context.get_retry_count(), 100);
        assert!(context.get_last_error().is_some());
        assert_eq!(policy.get_max_attempts(), 0);
    }
}
