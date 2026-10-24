//! The strong-reference retry context cache.

use std::{ops::Deref, sync::Arc};

use crate::{error::retry_error::RetryError, retry_context::RetryContext};

use super::{
    base_map_retry_context_cache::BaseMapRetryContextCache,
    retry_context_cache::{DEFAULT_CAPACITY, RetryContextCache},
};

/// The strong-reference [`RetryContextCache`].
///
/// It keeps a context alive until the retry block closes it, so the attempt count of a
/// stateful retry survives between calls. The cache is bounded: at capacity it either
/// evicts the least recently used context or rejects the insertion, depending on how it
/// was built.
#[derive(Clone)]
pub struct MapRetryContextCache {
    inner: BaseMapRetryContextCache<Arc<dyn RetryContext>>,
}

impl MapRetryContextCache {
    /// Creates a cache with [`DEFAULT_CAPACITY`] that evicts the least recently used
    /// context at capacity.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Creates a cache with the given capacity that evicts the least recently used context
    /// at capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_eviction(capacity, true)
    }

    /// Creates a cache with the given capacity.
    ///
    /// When `remove_eldest_entries` is `false`, a full cache rejects new contexts with
    /// [`RetryError::RetryCacheCapacityExceededError`] instead of silently forgetting an
    /// item that may still be retried.
    pub fn with_capacity_and_eviction(capacity: usize, remove_eldest_entries: bool) -> Self {
        Self {
            inner: BaseMapRetryContextCache::new(
                capacity,
                remove_eldest_entries,
                Arc::new(|context: Arc<dyn RetryContext>| context),
                Arc::new(|value: &Arc<dyn RetryContext>| Some(value.clone())),
            ),
        }
    }
}

impl Default for MapRetryContextCache {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for MapRetryContextCache {
    type Target = BaseMapRetryContextCache<Arc<dyn RetryContext>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl RetryContextCache for MapRetryContextCache {
    fn get(&self, key: &str) -> Option<Arc<dyn RetryContext>> {
        self.inner.get(key)
    }

    fn put(&self, key: &str, context: Arc<dyn RetryContext>) -> Result<(), RetryError> {
        self.inner.put(key, context)
    }

    fn remove(&self, key: &str) {
        self.inner.remove(key);
    }

    fn contains_key(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::MapRetryContextCache;
    use crate::{
        context::retry_context_support::RetryContextSupport,
        policy::retry_context_cache::RetryContextCache, retry_context::RetryContext,
    };

    #[test]
    fn get_returns_the_same_cached_context_object() {
        let cache = MapRetryContextCache::new();
        let context: Arc<dyn RetryContext> = Arc::new(RetryContextSupport::new());

        cache
            .put("item", context.clone())
            .expect("capacity is free");
        let cached = cache.get("item").expect("context should exist");

        assert!(Arc::ptr_eq(&context, &cached));
    }

    #[test]
    fn remove_forgets_the_entry() {
        let cache = MapRetryContextCache::with_capacity(8);
        let context: Arc<dyn RetryContext> = Arc::new(RetryContextSupport::new());
        cache.put("item", context).expect("capacity is free");

        cache.remove("item");

        assert!(!cache.contains_key("item"));
    }
}
