use std::{collections::HashMap, sync::Arc};

use next_web_core::async_trait;
use tokio::sync::Mutex;

use crate::error::retry_error::RetryError;

use super::classifier::Classifier;

#[derive(Clone)]
pub struct BinaryErrorClassifier<T = RetryError, C = bool> {
    traverse_causes: bool,
    default_value: Option<C>,
    classified: Arc<Mutex<HashMap<T, C>>>,
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
            classified: Arc::new(Mutex::new(type_map)),
            default_value: Some(default_value),
            traverse_causes,
        }
    }

    pub fn with_default_value(default_value: bool) -> Self {
        Self {
            traverse_causes: false,
            default_value: Some(default_value),
            classified: Arc::new(Mutex::new(HashMap::new()))
        }
    }

    pub fn with_retryable_errors_and_default_value(
        type_map: HashMap<RetryError, bool>,
        default_value: bool,
    ) -> Self {
        Self {
            traverse_causes: false,
            default_value: Some(default_value),
            classified: Arc::new(Mutex::new(type_map)),
        }
    }

    pub fn with_retryable_errors_collects_and_default_value(
        errors: impl IntoIterator<Item = RetryError>,
        default_value: bool,
    ) -> Self {
        let mut classifier = Self::with_default_value(default_value);
        let items = errors.into_iter();
        classifier.set_type_map(items.map(|key| (key,default_value )).collect::<HashMap<_, _>>());

        classifier
    }

    pub fn set_traverse_causes(&mut self, traverse_causes: bool) {
        self.traverse_causes = traverse_causes;
    }

    pub fn set_type_map(&mut self, type_map: HashMap<RetryError, bool>) {
        self.classified = Arc::new(Mutex::new(type_map));
    }
}


#[async_trait]
impl Classifier<RetryError, bool> for BinaryErrorClassifier {
    async fn classify(&self, classifiable: Option<&RetryError>) -> bool {
        if classifiable.is_none() {
            return self.default_value.unwrap_or_default();
        }

        let classifiable = classifiable.unwrap();
        if let Some(value) = self.classified.lock().await.get(&classifiable) {
            return *value;
        }

        let mut value: Option<bool> = Some(true);


        if let Some(val) = value.as_ref() {
            self.classified.lock().await.insert(classifiable.clone(), *val);
        }

        if value.is_none() {
            value = self.default_value.clone();
        }

        let classified = value.unwrap_or_default();

        if !self.traverse_causes {
            return classified;
        }

        if classified == self.default_value.unwrap_or_default() {
            let cause = classifiable;
           
            let mut i = 0;
            while i >= 0  && (classified == self.default_value.unwrap_or_default()) {
                if self.classified.lock().await.contains_key(&cause) {
                    return classified;
                }

                i += 1;
            }
        }

        classified
    }
}
