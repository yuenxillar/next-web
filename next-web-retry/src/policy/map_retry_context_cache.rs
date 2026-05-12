use std::{collections::HashMap, sync::Arc};

use crate::{policy::retry_context_cache::RetryContextCache, retry_context::RetryContext};

#[derive(Clone, Default)]
pub struct MapRetryContextCache {
    contexts: HashMap<String, Arc<dyn RetryContext>>,
}

impl MapRetryContextCache {
    pub fn new() -> Self {
        Self::default()
    }
}

impl RetryContextCache for MapRetryContextCache {
    fn get(&self, key: &str) -> Option<Arc<dyn RetryContext>> {
        self.contexts.get(key).cloned()
    }

    fn get_mut(&self, _key: &str) -> Option<&mut dyn RetryContext> {
        None
    }

    fn put(&mut self, key: &str, value: Arc<dyn RetryContext>) {
        self.contexts.insert(key.to_string(), value);
    }

    fn remove(&mut self, key: &str) {
        self.contexts.remove(key);
    }

    fn contains_key(&self, key: &str) -> bool {
        self.contexts.contains_key(key)
    }
}

#[cfg(test)]
mod tests {
    use crate::context::retry_context_support::RetryContextSupport;

    use super::*;

    #[test]
    fn get_returns_the_same_cached_context_object() {
        let mut cache = MapRetryContextCache::new();
        let context: Arc<dyn RetryContext> = Arc::new(RetryContextSupport::default());

        cache.put("item", context.clone());
        let cached = cache.get("item").expect("context should exist");

        assert!(Arc::ptr_eq(&context, &cached));
    }
}
