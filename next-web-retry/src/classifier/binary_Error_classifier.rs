use std::{collections::HashMap, sync::Arc};

use dashmap::DashMap;
use next_web_core::async_trait;

use crate::error::retry_error::RetryError;

use super::classifier::Classifier;

#[derive(Clone)]
pub struct BinaryErrorClassifier<T = RetryError, C = bool> {
    traverse_causes: bool,
    default_value: Option<C>,
    classified: Arc<DashMap<T, C>>,
}

impl BinaryErrorClassifier {
    pub fn default_classifier() -> Self {
        Self::with_default_value(true)
    }

    pub fn new(
        type_map: HashMap<RetryError, bool>,
        default_value: bool,
        traverse_causes: bool,
    ) -> Self {
        Self {
            classified: Arc::new(type_map.into_iter().collect()),
            default_value: Some(default_value),
            traverse_causes,
        }
    }

    pub fn with_default_value(default_value: bool) -> Self {
        Self {
            traverse_causes: false,
            default_value: Some(default_value),
            classified: Arc::new(DashMap::new()),
        }
    }

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

    pub fn with_retryable_errors_collects_and_default_value(
        errors: impl IntoIterator<Item = RetryError>,
        default_value: bool,
    ) -> Self {
        let mut classifier = Self::with_default_value(default_value);
        let items = errors.into_iter();
        classifier.set_type_map(
            items
                .map(|key| (key, !default_value))
                .collect::<HashMap<_, _>>(),
        );

        classifier
    }

    pub fn set_traverse_causes(&mut self, traverse_causes: bool) {
        self.traverse_causes = traverse_causes;
    }

    pub fn set_type_map(&mut self, type_map: HashMap<RetryError, bool>) {
        self.classified = Arc::new(type_map.into_iter().collect());
    }
}

#[async_trait]
impl Classifier<RetryError, bool> for BinaryErrorClassifier {
    async fn classify(&self, classifiable: Option<&RetryError>) -> bool {
        if classifiable.is_none() {
            return self.default_value.unwrap_or_default();
        }

        let classifiable = classifiable.unwrap();
        if let Some(value) = self.classified.get(classifiable) {
            return *value;
        }

        let classified = self.default_value.unwrap_or_default();

        classified
    }
}

#[cfg(test)]
mod tests {
    use crate::{classifier::classifier::Classifier, error::retry_error::RetryError};

    use super::BinaryErrorClassifier;

    #[tokio::test]
    async fn default_classifier_retries_unclassified_errors() {
        let classifier = BinaryErrorClassifier::default_classifier();

        assert!(
            classifier
                .classify(Some(&RetryError::Custom("any".to_string())))
                .await
        );
    }

    #[tokio::test]
    async fn whitelist_classifier_only_retries_configured_errors() {
        let classifier = BinaryErrorClassifier::with_retryable_errors_collects_and_default_value(
            [RetryError::Custom("retry".to_string())],
            false,
        );

        assert!(
            classifier
                .classify(Some(&RetryError::Custom("retry".to_string())))
                .await
        );
        assert!(
            !classifier
                .classify(Some(&RetryError::Custom("stop".to_string())))
                .await
        );
    }

    #[tokio::test]
    async fn blacklist_classifier_rejects_configured_errors() {
        let classifier = BinaryErrorClassifier::with_retryable_errors_collects_and_default_value(
            [RetryError::Custom("stop".to_string())],
            true,
        );

        assert!(
            !classifier
                .classify(Some(&RetryError::Custom("stop".to_string())))
                .await
        );
        assert!(
            classifier
                .classify(Some(&RetryError::Custom("retry".to_string())))
                .await
        );
    }
}
