use std::collections::BTreeMap;

use next_web_core::anys::any_value::AnyValue;

use crate::{
    chat::observation::ai_operation_metadata::AiOperationMetadata, util::key_value::KeyValue,
};

#[derive(Clone)]
pub struct ModelObservationContext<Q, R> {
    pub(crate) request: Q,
    pub(crate) operation_metadata: AiOperationMetadata,
    pub(crate) response: Option<R>,

    /// impl context
    pub(crate) map: dashmap::DashMap<String, AnyValue>,
    pub(crate) name: Option<String>,
    pub(crate) contextual_name: Option<String>,

    pub(crate) low_cardinality_key_values: BTreeMap<String, Box<dyn KeyValue>>,
    pub(crate) high_cardinality_key_values: BTreeMap<String, Box<dyn KeyValue>>,
}

impl<Q, R> ModelObservationContext<Q, R> {
    pub fn new(request: Q, operation_metadata: AiOperationMetadata) -> Self {
        Self {
            request,
            operation_metadata,
            response: None,
            map: Default::default(),
            name: Default::default(),
            contextual_name: Default::default(),
            low_cardinality_key_values: Default::default(),
            high_cardinality_key_values: Default::default(),
        }
    }
}

impl<Q, R> ModelObservationContext<Q, R> {
    pub fn operation_metadata(&self) -> &AiOperationMetadata {
        &self.operation_metadata
    }

    pub fn request(&self) -> &Q {
        &self.request
    }

    pub fn response(&self) -> Option<&R> {
        self.response.as_ref()
    }
    pub fn set_response(&mut self, response: R) {
        self.response = Some(response);
    }
}

impl<Q, R> ModelObservationContext<Q, R> {
    pub fn name(&self) -> Option<&str> {
        self.name.as_ref().map(|x| x.as_str())
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = Some(name.into());
    }

    pub fn contextual_name(&self) -> Option<&str> {
        self.contextual_name.as_ref().map(|x| x.as_str())
    }

    pub fn set_contextual_name(&mut self, contextual_name: impl Into<String>) {
        self.contextual_name = Some(contextual_name.into());
    }

    pub fn put(&self, key: impl Into<String>, value: impl Into<AnyValue>) {
        self.map.insert(key.into(), value.into());
    }

    pub fn get(&self, key: impl AsRef<str>) -> Option<AnyValue> {
        self.map.get(key.as_ref()).map(|v| v.value().clone())
    }

    pub fn get_or_default(&self, key: impl AsRef<str>, default: impl Into<AnyValue>) -> AnyValue {
        self.map
            .get(key.as_ref())
            .map(|v| v.value().clone())
            .unwrap_or(default.into())
    }

    pub fn contains_key(&self, key: impl AsRef<str>) -> bool {
        self.map.contains_key(key.as_ref())
    }

    pub fn remove(&self, key: impl AsRef<str>) -> Option<(String, AnyValue)> {
        self.map.remove(key.as_ref())
    }

    pub fn clear(&self) {
        self.map.clear()
    }
}
