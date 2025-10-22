use crate::anys::{any_map::AnyMap, any_value::AnyValue};

#[derive(Clone)]
pub struct JobExecutionContext {
    pub(crate) data_map: AnyMap,
}

impl JobExecutionContext {
    pub async fn put<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<AnyValue>,
    {
        self.data_map.set(key.into(), value.into()).await;
    }
}
impl Default for JobExecutionContext {
    fn default() -> Self {
        Self {
            data_map: AnyMap::default(),
        }
    }
}
