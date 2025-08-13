use next_web_core::util::any_map::AnyValue;

use crate::chat::observation::ai_operation_metadata::AiOperationMetadata;

#[derive(Clone)]
pub struct ModelObservationContext<Q, R> {
    request: Q,
    operation_metadata: AiOperationMetadata,
    response: R,

    /// impl context
    map: dashmap::DashMap<String, AnyValue>,
    name: String,
    contextual_name: String,
}

impl<Q, R> ModelObservationContext<Q, R> {
    pub fn operation_metadata(&self) -> &AiOperationMetadata {
        &self.operation_metadata
    }

    pub fn request(&self) -> &Q {
        &self.request
    }

    pub fn response(&self) -> &R {
        &self.response
    }
    pub fn set_response(&mut self, response: R) {
        self.response = response;
    }
}

impl<Q, R> ModelObservationContext<Q, R> {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    pub fn contextual_name(&self) -> &str {
        self.contextual_name.as_str()
    }

    pub fn set_contextual_name(&mut self, contextual_name: impl Into<String>) {
        self.contextual_name = contextual_name.into();
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
