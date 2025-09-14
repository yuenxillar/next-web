use std::collections::HashMap;

use crate::{classifier::classifier::Classifier, error::retry_error::RetryError};

#[derive(Clone)]
pub struct BinaryErrorClassifier<T = RetryError, C = bool> {
    traverse_causes: bool,
    default_value: Option<C>,
    classified: HashMap<T, C>,
}

impl BinaryErrorClassifier {
    pub fn default_classifier() -> Self {
        let mut map: HashMap<RetryError, bool> = Default::default();
        map.insert(RetryError::Custom("TODO".to_string()), true);
        Self::with_retryable_errors_and_default_value(map, false)
    }

    pub fn new(
        type_map: HashMap<RetryError, bool>,
        default_value: bool,
        traverse_causes: bool,
    ) -> Self {
        Self {
            classified: type_map,
            default_value: Some(default_value),
            traverse_causes,
        }
    }

    pub fn with_retryable_errors_and_default_value(
        type_map: HashMap<RetryError, bool>,
        default_value: bool,
    ) -> Self {
        Self {
            traverse_causes: false,
            default_value: Some(default_value),
            classified: type_map,
        }
    }

    pub fn set_traverse_causes(&mut self, traverse_causes: bool) {
        self.traverse_causes = traverse_causes;
    }
}

impl<T, C> Classifier<T, C> for BinaryErrorClassifier {
    fn classify(&self, classifiable: &C) -> T {
        todo!()
    }
}
