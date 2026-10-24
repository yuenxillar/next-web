//! Classifies an error as retryable or not.

use std::{collections::HashMap, hash::Hash, sync::Arc};

use dashmap::DashMap;

use crate::error::retry_error::RetryError;

use super::classifier::Classifier;

/// The maximum depth walked by cause traversal.
///
/// A cyclic or pathologically deep cause chain must not turn classification into an
/// infinite loop, so traversal stops after this many links.
const MAX_CAUSE_DEPTH: usize = 32;

/// Classifies an error as retryable or not.
///
/// The classifier stores an explicit answer per error and a default for everything else.
/// Errors are matched by [`RetryError`] equality, which compares the error kind and
/// message, so a policy configured with a representative error also matches concrete
/// failures that render identically.
///
/// When [`BinaryErrorClassifier::set_traverse_causes`] is enabled and the error itself has
/// no explicit answer, the cause chain is walked until an answer is found.
#[derive(Clone)]
pub struct BinaryErrorClassifier<T = RetryError, C = bool> {
    traverse_causes: bool,
    default_value: Option<C>,
    classified: Arc<DashMap<T, C>>,
}

impl BinaryErrorClassifier<RetryError, bool> {
    /// Creates a classifier that answers `true` for every unclassified error.
    pub fn default_classifier() -> Self {
        Self::with_default_value(true)
    }

    /// Creates a classifier from an explicit map.
    pub fn new(
        type_map: HashMap<RetryError, bool>,
        default_value: bool,
        traverse_causes: bool,
    ) -> Self {
        Self {
            traverse_causes,
            default_value: Some(default_value),
            classified: Arc::new(type_map.into_iter().collect()),
        }
    }

    /// Creates a classifier with an explicit default and no entries.
    pub fn with_default_value(default_value: bool) -> Self {
        Self {
            traverse_causes: false,
            default_value: Some(default_value),
            classified: Arc::new(DashMap::new()),
        }
    }

    /// Creates a classifier from a map and a default, without cause traversal.
    pub fn with_retryable_errors_and_default_value(
        type_map: HashMap<RetryError, bool>,
        default_value: bool,
    ) -> Self {
        Self {
            traverse_causes: false,
            default_value: Some(default_value),
            classified: Arc::new(type_map.into_iter().collect()),
        }
    }

    /// Creates a classifier whose entries all answer the opposite of `default_value`.
    ///
    /// This is the convenient constructor for a white list (`default_value = false` means
    /// only the listed errors are retried) and for a black list (`default_value = true`
    /// means only the listed errors are not retried).
    pub fn with_retryable_errors_collects_and_default_value(
        errors: impl IntoIterator<Item = RetryError>,
        default_value: bool,
    ) -> Self {
        let mut classifier = Self::with_default_value(default_value);
        classifier.set_type_map(
            errors
                .into_iter()
                .map(|key| (key, !default_value))
                .collect::<HashMap<_, _>>(),
        );

        classifier
    }

    /// Enables or disables cause traversal.
    pub fn set_traverse_causes(&mut self, traverse_causes: bool) {
        self.traverse_causes = traverse_causes;
    }

    /// Returns whether cause traversal is enabled.
    pub fn is_traverse_causes(&self) -> bool {
        self.traverse_causes
    }

    /// Replaces the explicit answers.
    pub fn set_type_map(&mut self, type_map: HashMap<RetryError, bool>) {
        self.classified = Arc::new(type_map.into_iter().collect());
    }

    /// Adds or replaces the answer for one error.
    pub fn classify_as(&self, error: RetryError, retryable: bool) {
        self.classified.insert(error, retryable);
    }

    fn lookup(&self, error: &RetryError) -> Option<bool> {
        self.classified.get(error).map(|value| *value)
    }
}

impl Classifier<RetryError, bool> for BinaryErrorClassifier<RetryError, bool> {
    fn classify(&self, classifiable: Option<&RetryError>) -> bool {
        let Some(error) = classifiable else {
            return self.default_value.unwrap_or_default();
        };

        if let Some(answer) = self.lookup(error) {
            return answer;
        }

        if self.traverse_causes {
            let mut current = error
                .cause()
                .map(|cause| RetryError::from(cause.to_boxed()));
            let mut depth = 0_usize;
            while let Some(cause) = current {
                if depth >= MAX_CAUSE_DEPTH {
                    break;
                }
                depth += 1;
                if let Some(answer) = self.lookup(&cause) {
                    return answer;
                }
                current = cause
                    .cause()
                    .map(|cause| RetryError::from(cause.to_boxed()));
            }
            return self.default_value.unwrap_or_default();
        }

        self.default_value.unwrap_or_default()
    }
}

impl<T, C> BinaryErrorClassifier<T, C>
where
    T: Eq + Hash + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
{
    /// Returns the number of explicit answers.
    pub fn len(&self) -> usize {
        self.classified.len()
    }

    /// Returns whether the classifier has no explicit answers.
    pub fn is_empty(&self) -> bool {
        self.classified.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::BinaryErrorClassifier;
    use crate::{classifier::classifier::Classifier, error::retry_error::RetryError};

    #[test]
    fn default_classifier_retries_unclassified_errors() {
        let classifier = BinaryErrorClassifier::default_classifier();

        assert!(classifier.classify(Some(&RetryError::custom("any"))));
        assert!(classifier.classify(None));
    }

    #[test]
    fn whitelist_classifier_only_retries_configured_errors() {
        let classifier = BinaryErrorClassifier::with_retryable_errors_collects_and_default_value(
            [RetryError::custom("retry")],
            false,
        );

        assert!(classifier.classify(Some(&RetryError::custom("retry"))));
        assert!(!classifier.classify(Some(&RetryError::custom("stop"))));
    }

    #[test]
    fn blacklist_classifier_rejects_configured_errors() {
        let classifier = BinaryErrorClassifier::with_retryable_errors_collects_and_default_value(
            [RetryError::custom("stop")],
            true,
        );

        assert!(!classifier.classify(Some(&RetryError::custom("stop"))));
        assert!(classifier.classify(Some(&RetryError::custom("retry"))));
    }

    #[test]
    fn cause_traversal_finds_a_nested_answer() {
        let mut classifier = BinaryErrorClassifier::with_retryable_errors_and_default_value(
            [(RetryError::custom("inner"), false)].into_iter().collect(),
            true,
        );
        classifier.set_traverse_causes(true);
        let error = RetryError::retry_with_cause("outer", Box::new(RetryError::custom("inner")));

        assert!(!classifier.classify(Some(&error)));
    }

    #[test]
    fn cause_traversal_is_opt_in() {
        let classifier = BinaryErrorClassifier::with_retryable_errors_and_default_value(
            [(RetryError::custom("inner"), false)].into_iter().collect(),
            true,
        );
        let error = RetryError::retry_with_cause("outer", Box::new(RetryError::custom("inner")));

        assert!(classifier.classify(Some(&error)));
    }
}
