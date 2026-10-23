//! A policy that decides with a predicate over the last error.

use std::sync::Arc;

use next_web_core::Named;

use crate::{
    Predicate, context::retry_context_support::RetryContextSupport, error::retry_error::RetryError,
    retry_context::RetryContext, retry_policy::RetryPolicy,
};

/// A policy that decides with a predicate over the last error.
///
/// Use it when classification needs more than a white or black list: a predicate can look
/// at anything the error exposes. Like
/// [`BinaryErrorClassifierRetryPolicy`](super::binary_error_classifier_retry_policy::BinaryErrorClassifierRetryPolicy)
/// it does not count attempts, so compose it with a counting policy for a bounded retry.
#[derive(Clone)]
pub struct PredicateRetryPolicy {
    predicate: Arc<dyn Predicate<RetryError>>,
}

impl PredicateRetryPolicy {
    /// Creates a policy from a predicate.
    pub fn new(predicate: Arc<dyn Predicate<RetryError>>) -> Self {
        Self { predicate }
    }

    /// Returns the predicate of this policy.
    pub fn predicate(&self) -> &Arc<dyn Predicate<RetryError>> {
        &self.predicate
    }
}

impl RetryPolicy for PredicateRetryPolicy {
    fn can_retry(&self, context: &dyn RetryContext) -> bool {
        match context.get_last_error() {
            Some(error) => self.predicate.test(&error),
            None => true,
        }
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

impl Named for PredicateRetryPolicy {
    fn name(&self) -> &str {
        "PredicateRetryPolicy"
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::PredicateRetryPolicy;
    use crate::{error::retry_error::RetryError, retry_policy::RetryPolicy};

    #[test]
    fn the_first_attempt_is_always_allowed() {
        let policy = PredicateRetryPolicy::new(Arc::new(|_: &RetryError| false));
        let context = policy.open(None);

        assert!(policy.can_retry(context.as_ref()));
    }

    #[test]
    fn the_predicate_sees_the_last_error() {
        let policy =
            PredicateRetryPolicy::new(Arc::new(|error: &RetryError| error.message() == "failed"));
        let context = policy.open(None);
        policy.register_error(context.as_ref(), &RetryError::custom("failed"));

        assert_eq!(context.get_retry_count(), 1);
        assert!(policy.can_retry(context.as_ref()));
    }

    #[test]
    fn a_rejecting_predicate_stops_the_retry() {
        let policy = PredicateRetryPolicy::new(Arc::new(|_: &RetryError| false));
        let context = policy.open(None);
        policy.register_error(context.as_ref(), &RetryError::custom("failed"));

        assert!(!policy.can_retry(context.as_ref()));
    }
}
