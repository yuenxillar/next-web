//! A retry context cache that does not keep its contexts alive.

use std::{
    ops::Deref,
    sync::{Arc, Weak},
};

use crate::{error::retry_error::RetryError, retry_context::RetryContext};

use super::{
    base_map_retry_context_cache::BaseMapRetryContextCache,
    retry_context_cache::{DEFAULT_CAPACITY, RetryContextCache},
};

/// A [`RetryContextCache`] that does not keep its contexts alive.
///
/// The cache stores a weak reference per key. As long as something else holds the context
/// the entry works exactly like [`MapRetryContextCache`](super::map_retry_context_cache::MapRetryContextCache);
/// once the last strong reference is dropped the entry is dead and the cache drops it on
/// the next access. This is the Rust-native counterpart of a soft-reference cache: the
/// retry state yields under memory pressure instead of pinning contexts forever.
#[derive(Clone)]
pub struct SoftReferenceMapRetryContextCache {
    inner: BaseMapRetryContextCache<Weak<dyn RetryContext>>,
}

impl SoftReferenceMapRetryContextCache {
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
    pub fn with_capacity_and_eviction(capacity: usize, remove_eldest_entries: bool) -> Self {
        Self {
            inner: BaseMapRetryContextCache::new(
                capacity,
                remove_eldest_entries,
                Arc::new(|context: Arc<dyn RetryContext>| Arc::downgrade(&context)),
                Arc::new(|value: &Weak<dyn RetryContext>| value.upgrade()),
            ),
        }
    }
}

impl Default for SoftReferenceMapRetryContextCache {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for SoftReferenceMapRetryContextCache {
    type Target = BaseMapRetryContextCache<Weak<dyn RetryContext>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl RetryContextCache for SoftReferenceMapRetryContextCache {
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

    use super::SoftReferenceMapRetryContextCache;
    use crate::{
        context::retry_context_support::RetryContextSupport,
        policy::retry_context_cache::RetryContextCache, retry_context::RetryContext,
    };

    #[test]
    fn a_live_context_is_returned() {
        let cache = SoftReferenceMapRetryContextCache::new();
        let context: Arc<dyn RetryContext> = Arc::new(RetryContextSupport::new());
        cache
            .put("item", context.clone())
            .expect("capacity is free");

        assert!(cache.get("item").is_some());
        assert!(cache.contains_key("item"));
    }

    #[test]
    fn a_dropped_context_is_forgotten() {
        let cache = SoftReferenceMapRetryContextCache::new();
        {
            let context: Arc<dyn RetryContext> = Arc::new(RetryContextSupport::new());
            cache.put("item", context).expect("capacity is free");
        }

        assert!(cache.get("item").is_none());
        assert!(!cache.contains_key("item"));
        assert_eq!(cache.len(), 0);
    }
}
