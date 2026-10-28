//! Classifies an error into an arbitrary value.

use std::{collections::HashMap, sync::Arc};

use dashmap::DashMap;

use crate::error::retry_error::RetryError;

use super::classifier::Classifier;

/// Classifies an error into an arbitrary value, with a default for unknown errors.
///
/// This is the general form of
/// [`BinaryErrorClassifier`](super::binary_error_classifier::BinaryErrorClassifier): instead
/// of a boolean it returns
/// a value, which is how
/// [`ExceptionClassifierRetryPolicy`](crate::policy::exception_classifier_retry_policy::ExceptionClassifierRetryPolicy)
/// picks the policy to apply for a given error.
pub struct MapClassifier<C> {
    classified: DashMap<RetryError, C>,
    default: C,
}

impl<C> MapClassifier<C>
where
    C: Clone + Send + Sync,
{
    /// Creates a classifier from a map and a default value.
    pub fn new(classifications: HashMap<RetryError, C>, default: C) -> Self {
        Self {
            classified: classifications.into_iter().collect(),
            default,
        }
    }

    /// Creates a classifier with only a default.
    pub fn with_default(default: C) -> Self {
        Self {
            classified: DashMap::new(),
            default,
        }
    }

    /// Adds or replaces the classification of one error.
    pub fn classify_as(&self, error: RetryError, value: C) {
        self.classified.insert(error, value);
    }

    /// Returns the number of explicit classifications.
    pub fn len(&self) -> usize {
        self.classified.len()
    }

    /// Returns whether there are no explicit classifications.
    pub fn is_empty(&self) -> bool {
        self.classified.is_empty()
    }
}

impl<C> Classifier<RetryError, C> for MapClassifier<C>
where
    C: Clone + Send + Sync,
{
    fn classify(&self, classifiable: Option<&RetryError>) -> C {
        classifiable
            .and_then(|error| self.classified.get(error).map(|value| value.clone()))
            .unwrap_or_else(|| self.default.clone())
    }
}

impl<C> Clone for MapClassifier<C>
where
    C: Clone + Send + Sync,
{
    fn clone(&self) -> Self {
        Self {
            classified: self
                .classified
                .iter()
                .map(|entry| (entry.key().clone(), entry.value().clone()))
                .collect(),
            default: self.default.clone(),
        }
    }
}

/// A classifier that always returns the same value.
pub struct ConstantClassifier<C> {
    value: C,
}

impl<C> ConstantClassifier<C> {
    /// Creates a classifier that returns `value` for every error.
    pub fn new(value: C) -> Self {
        Self { value }
    }
}

impl<C> Classifier<RetryError, C> for ConstantClassifier<C>
where
    C: Clone + Send + Sync,
{
    fn classify(&self, _classifiable: Option<&RetryError>) -> C {
        self.value.clone()
    }
}

impl<C: Clone + Send + Sync> Clone for ConstantClassifier<C> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}

/// Erases the value type of a [`Classifier`] behind an `Arc`.
pub type BoxedClassifier<C> = Arc<dyn Classifier<RetryError, C>>;

#[cfg(test)]
mod tests {
    use super::{ConstantClassifier, MapClassifier};
    use crate::{classifier::classifier::Classifier, error::retry_error::RetryError};

    #[test]
    fn a_map_classifier_uses_the_default_for_unknown_errors() {
        let classifier = MapClassifier::new(
            [(RetryError::custom("known"), "known")]
                .into_iter()
                .collect(),
            "other",
        );

        assert_eq!(
            classifier.classify(Some(&RetryError::custom("known"))),
            "known"
        );
        assert_eq!(
            classifier.classify(Some(&RetryError::custom("unknown"))),
            "other"
        );
        assert_eq!(classifier.classify(None), "other");
        assert_eq!(classifier.len(), 1);
    }

    #[test]
    fn a_constant_classifier_ignores_the_error() {
        let classifier = ConstantClassifier::new(7_u32);

        assert_eq!(classifier.classify(Some(&RetryError::custom("any"))), 7);
    }
}
