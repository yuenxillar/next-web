use std::{borrow::Cow, sync::Arc};

use tokio::sync::Mutex;

use crate::anys::{any_map::AnyMap, any_value::AnyValue};

#[derive(Clone)]
pub struct JobExecutionContext {
    pub(crate) any_map: Arc<Mutex<AnyMap>>,
}

impl JobExecutionContext {
    pub async fn put<K, V>(&mut self, key: K, value: V)
    where
        K: Into<Cow<'static, str>>,
        V: Into<AnyValue>,
    {
        self.any_map.lock().await.set(key.into(), value.into());
    }
}
impl Default for JobExecutionContext {
    fn default() -> Self {
        Self {
            any_map: Arc::new(Mutex::new(AnyMap::new())),
        }
    }
}
