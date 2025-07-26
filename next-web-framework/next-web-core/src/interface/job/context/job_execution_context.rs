use std::borrow::Cow;

use crate::utils::any_map::{AnyMap, AnyValue};

#[derive(Clone)]
pub struct JobExecutionContext {
    pub(crate) any_map: AnyMap,
}

impl JobExecutionContext {
    pub fn put<K, V>(&mut self, key: K, value: V)
    where
        K: Into<Cow<'static, str>>,
        V: Into<AnyValue>,
    {
        self.any_map.set(key.into(), value.into());
    }
}
impl Default for JobExecutionContext {
    fn default() -> Self {
        Self {
            any_map: AnyMap::new(),
        }
    }
}
