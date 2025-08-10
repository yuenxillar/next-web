use crate::util::key_value::NoneKeyValue;

use super::key_value::KeyValue;

#[derive(Clone)]
pub struct KeyName(pub String);

impl KeyName {
    pub fn merge<I>(key_names: I) -> Vec<Self>
    where
        I: IntoIterator<Item = Vec<Self>>,
        Self: Sized,
    {
        key_names.into_iter().flatten().collect()
    }

    pub fn with_value(&self, value: impl Into<String>) -> impl KeyValue {
        NoneKeyValue::of_immutable(self.as_string(), value.into())
    }

    pub fn as_string(&self) -> String {
        self.0.clone()
    }

    pub fn is_required(&self) -> bool {
        true
    }
}
