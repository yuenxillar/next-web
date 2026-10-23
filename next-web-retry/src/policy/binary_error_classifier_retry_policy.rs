//! A policy that retries only the errors a classifier accepts.

use std::sync::Arc;

use next_web_core::Named;

use crate::{
    classifier::{binary_error_classifier::BinaryErrorClassifier, classifier::Classifier},
    context::retry_context_support::RetryContextSupport,
    error::retry_error::RetryError,
    retry_context::RetryContext,
    retry_policy::RetryPolicy,
};

/// A policy that decides by classifying the last error as retryable or not.
///
/// It does not count attempts; compose it with
/// [`MaxAttemptsRetryPolicy`](super::max_attempts_retry_policy::MaxAttemptsRetryPolicy) to
/// bound the retry. Binary classification is enough for most retries; use
/// [`ExceptionClassifierRetryPolicy`](super::exception_classifier_retry_policy::ExceptionClassifierRetryPolicy)
/// when different errors need different policies.
#[derive(Clone)]
pub struct BinaryErrorClassifierRetryPolicy {
    exception_classifier: BinaryErrorClassifier,
}

impl BinaryErrorClassifierRetryPolicy {
    /// Creates a policy from a classifier.
    pub fn new(exception_classifier: BinaryErrorClassifier) -> Self {
        Self {
            exception_classifier,
        }
    }

    /// Returns the classifier this policy delegates to.
    pub fn get_exception_classifier(&self) -> &BinaryErrorClassifier {
        &self.exception_classifier
    }
}

impl RetryPolicy for BinaryErrorClassifierRetryPolicy {
    fn can_retry(&self, context: &dyn RetryContext) -> bool {
        let last_error = context.get_last_error();
        last_error.is_none() || self.exception_classifier.classify(last_error.as_ref())
    }

    fn open(&self, parent: Option<Arc<dyn RetryContext>>) -> Arc<dyn RetryContext> {
        Arc::new(RetryContextSupport::with_parent(parent))
    }

    fn close(&self, _context: &dyn RetryContext) {}

    fn register_error(&self, context: &dyn RetryContext, error: &RetryError) {
        let any: &dyn std::any::Any = context;
        if let Some(support) = any.downcast_ref::<RetryContextSupport>() {
            support.register_error(error);
        }
    }
}

impl Named for BinaryErrorClassifierRetryPolicy {
    fn name(&self) -> &str {
        "BinaryErrorClassifierRetryPolicy"
    }
}

#[cfg(test)]
mod tests {
    use super::BinaryErrorClassifierRetryPolicy;
    use crate::{
        classifier::binary_error_classifier::BinaryErrorClassifier, error::retry_error::RetryError,
        retry_policy::RetryPolicy,
    };

    #[test]
    fn the_error_controls_whether_another_attempt_is_allowed() {
        let policy = BinaryErrorClassifierRetryPolicy::new(
            BinaryErrorClassifier::with_retryable_errors_collects_and_default_value(
                [RetryError::custom("retry")],
                false,
            ),
        );
        let context = policy.open(None);

        assert!(policy.can_retry(context.as_ref()));
        policy.register_error(context.as_ref(), &RetryError::custom("retry"));
        assert!(policy.can_retry(context.as_ref()));
        policy.register_error(context.as_ref(), &RetryError::custom("stop"));
        assert!(!policy.can_retry(context.as_ref()));
        assert_eq!(context.get_retry_count(), 2);
    }

    #[test]
    fn the_default_classifier_retries_any_error_without_a_limit() {
        let policy =
            BinaryErrorClassifierRetryPolicy::new(BinaryErrorClassifier::default_classifier());
        let context = policy.open(None);

        for _ in 0..10 {
            policy.register_error(context.as_ref(), &RetryError::custom("any"));
            assert!(policy.can_retry(context.as_ref()));
        }

        assert_eq!(policy.get_max_attempts(), 0);
    }
}
