//! A policy that retries a fixed number of times for a set of errors.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use next_web_core::Named;

use crate::{
    classifier::{binary_error_classifier::BinaryErrorClassifier, classifier::Classifier},
    context::retry_context_support::RetryContextSupport,
    error::retry_error::RetryError,
    retry_context::{RetryContext, retry_context_constants},
    retry_policy::RetryPolicy,
};

/// The default number of attempts of a new policy.
pub const DEFAULT_MAX_ATTEMPTS: u16 = 3;

/// A policy that retries a fixed number of times for a set of errors.
///
/// The attempt count includes the initial try, so a policy built with `3` runs the callback
/// at least once and at most three times.
///
/// The policy also distinguishes errors that must never reach a recoverer: when
/// [`SimpleRetryPolicy::set_not_recoverable`] names an error, the context is marked with
/// [`retry_context_constants::NO_RECOVERY`] as soon as the retry stops on it, and the
/// template then rethrows instead of calling the recovery callback.
#[derive(Clone)]
pub struct SimpleRetryPolicy {
    max_attempts: Arc<AtomicU64>,
    max_attempts_supplier: Option<Arc<dyn Fn() -> u16 + Send + Sync>>,
    retryable_classifier: BinaryErrorClassifier,
    recoverable_classifier: BinaryErrorClassifier,
}

impl SimpleRetryPolicy {
    /// Creates a policy with an explicit classifier.
    ///
    /// `retryable_errors` maps an error to whether it should be retried. Errors that are
    /// not in the map are answered with `default_value`. When `traverse_causes` is `true`
    /// the cause chain is inspected before falling back to the default.
    pub fn new(
        max_attempts: u16,
        retryable_errors: std::collections::HashMap<RetryError, bool>,
        traverse_causes: bool,
        default_value: bool,
    ) -> Self {
        let mut retryable_classifier =
            BinaryErrorClassifier::with_retryable_errors_and_default_value(
                retryable_errors,
                default_value,
            );
        retryable_classifier.set_traverse_causes(traverse_causes);
        Self {
            max_attempts: Arc::new(AtomicU64::new(u64::from(max_attempts))),
            max_attempts_supplier: None,
            retryable_classifier,
            recoverable_classifier: BinaryErrorClassifier::new(
                std::collections::HashMap::with_capacity(1),
                true,
                true,
            ),
        }
    }

    /// Creates a policy that retries every error.
    pub fn with_max_attempts(max_attempts: u16) -> Self {
        Self::with_binary_classifier(max_attempts, BinaryErrorClassifier::default_classifier())
    }

    /// Creates a policy from a map of retryable errors.
    pub fn with_retryable_errors(
        max_attempts: u16,
        retryable_errors: std::collections::HashMap<RetryError, bool>,
    ) -> Self {
        Self::with_retryable_errors_and_traverse_causes(max_attempts, retryable_errors, false)
    }

    /// Creates a policy from a map of retryable errors, inspecting cause chains.
    pub fn with_retryable_errors_and_traverse_causes(
        max_attempts: u16,
        retryable_errors: std::collections::HashMap<RetryError, bool>,
        traverse_causes: bool,
    ) -> Self {
        Self::new(max_attempts, retryable_errors, traverse_causes, false)
    }

    /// Creates a policy from a fully configured classifier.
    pub fn with_binary_classifier(max_attempts: u16, classifier: BinaryErrorClassifier) -> Self {
        Self {
            max_attempts: Arc::new(AtomicU64::new(u64::from(max_attempts))),
            max_attempts_supplier: None,
            retryable_classifier: classifier,
            recoverable_classifier: BinaryErrorClassifier::new(
                std::collections::HashMap::with_capacity(1),
                true,
                true,
            ),
        }
    }

    /// Updates the number of attempts.
    pub fn set_max_attempts(&self, max_attempts: u16) {
        self.max_attempts
            .store(u64::from(max_attempts), Ordering::Release);
    }

    /// Marks errors that must not be passed to a recoverer.
    ///
    /// A retry that stops on one of them is rethrown immediately instead of being
    /// recovered.
    pub fn set_not_recoverable(&mut self, no_recovery: Vec<RetryError>) {
        let type_map = no_recovery
            .into_iter()
            .map(|error| (error, false))
            .collect::<std::collections::HashMap<_, _>>();
        self.recoverable_classifier = BinaryErrorClassifier::new(type_map, true, true);
    }

    /// Installs a dynamic source for the number of attempts.
    ///
    /// The supplier is called on every check, which allows the limit to follow live
    /// configuration. Note that a policy with a supplier cannot be serialised into a
    /// distributed cache.
    pub fn set_max_attempts_supplier<F>(&mut self, max_attempts_supplier: F)
    where
        F: Fn() -> u16 + Send + Sync + 'static,
    {
        self.max_attempts_supplier = Some(Arc::new(max_attempts_supplier));
    }

    /// Returns the classifier that decides whether an error is retryable.
    pub fn retryable_classifier(&self) -> &BinaryErrorClassifier {
        &self.retryable_classifier
    }
}

impl Default for SimpleRetryPolicy {
    fn default() -> Self {
        Self::with_max_attempts(DEFAULT_MAX_ATTEMPTS)
    }
}

impl RetryPolicy for SimpleRetryPolicy {
    fn can_retry(&self, context: &dyn RetryContext) -> bool {
        let error = context.get_last_error();
        let retryable = match &error {
            None => true,
            Some(error) => self.retryable_classifier.classify(Some(error)),
        };
        let can_retry = retryable && context.get_retry_count() < self.get_max_attempts();

        let recoverable = match &error {
            Some(error) => self.recoverable_classifier.classify(Some(error)),
            None => true,
        };

        if !can_retry && error.is_some() && !recoverable {
            context.set_attribute(retry_context_constants::NO_RECOVERY, true.into());
        } else {
            context.remove_attribute(retry_context_constants::NO_RECOVERY);
        }

        can_retry
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
        if let Some(supplier) = self.max_attempts_supplier.as_ref() {
            return supplier();
        }
        u16::try_from(self.max_attempts.load(Ordering::Acquire)).unwrap_or(u16::MAX)
    }
}

impl Named for SimpleRetryPolicy {
    fn name(&self) -> &str {
        "SimpleRetryPolicy"
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::SimpleRetryPolicy;
    use crate::{
        error::retry_error::RetryError, retry_context::retry_context_constants,
        retry_policy::RetryPolicy,
    };

    #[test]
    fn attempts_are_bounded_and_all_errors_are_retried_by_default() {
        let policy = SimpleRetryPolicy::default();
        let context = policy.open(None);

        for count in 0..2 {
            assert!(policy.can_retry(context.as_ref()));
            policy.register_error(context.as_ref(), &RetryError::custom("failed"));
            assert_eq!(
                context.get_retry_count(),
                count + 1,
                "the count must track the failures"
            );
        }
        assert!(policy.can_retry(context.as_ref()));
        policy.register_error(context.as_ref(), &RetryError::custom("failed"));
        assert!(!policy.can_retry(context.as_ref()));
    }

    #[test]
    fn a_non_retryable_error_stops_immediately() {
        let policy = SimpleRetryPolicy::with_retryable_errors(
            5,
            HashMap::from([(RetryError::custom("fatal"), false)]),
        );
        let context = policy.open(None);
        policy.register_error(context.as_ref(), &RetryError::custom("fatal"));

        assert!(!policy.can_retry(context.as_ref()));
        // Recovery stays enabled: only an explicitly non-recoverable error disables it.
        assert!(!context.is_no_recovery());
    }

    #[test]
    fn a_retryable_error_keeps_going() {
        let policy = SimpleRetryPolicy::with_retryable_errors(
            5,
            HashMap::from([(RetryError::custom("transient"), true)]),
        );
        let context = policy.open(None);
        policy.register_error(context.as_ref(), &RetryError::custom("transient"));

        assert!(policy.can_retry(context.as_ref()));
        assert!(!context.is_no_recovery());
    }

    #[test]
    fn a_not_recoverable_error_is_marked_on_the_context() {
        let mut policy = SimpleRetryPolicy::new(1, HashMap::new(), false, true);
        policy.set_not_recoverable(vec![RetryError::custom("fatal")]);
        let context = policy.open(None);
        policy.register_error(context.as_ref(), &RetryError::custom("fatal"));

        assert!(!policy.can_retry(context.as_ref()));
        assert_eq!(
            context
                .get_attribute(retry_context_constants::NO_RECOVERY)
                .and_then(|value| value.as_boolean()),
            Some(true)
        );
    }

    #[test]
    fn a_max_attempts_supplier_overrides_the_stored_limit() {
        let mut policy = SimpleRetryPolicy::with_max_attempts(1);
        policy.set_max_attempts_supplier(|| 7);

        assert_eq!(policy.get_max_attempts(), 7);
    }
}
