//! The bounded, in-memory base of the retry context caches.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use indexmap::IndexMap;
use tracing::warn;

use crate::{error::retry_error::RetryError, retry_context::RetryContext};

use super::retry_context_cache::{DEFAULT_CAPACITY, RetryContextCache};

/// Converts a retry context into the value a cache stores.
pub type ToValue<V> = Arc<dyn Fn(Arc<dyn RetryContext>) -> V + Send + Sync>;

/// Recovers a retry context from the value a cache stores.
///
/// Returning `None` means the entry is no longer usable (for example a dead weak
/// reference) and the cache drops it.
pub type FromValue<V> = Arc<dyn Fn(&V) -> Option<Arc<dyn RetryContext>> + Send + Sync>;

/// An in-memory [`RetryContextCache`] with a bounded capacity.
///
/// The base implementation is generic over the stored value so the same bookkeeping
/// serves both the strong-reference cache and the cache that may lose entries. Entries are
/// ordered by use: reading an entry moves it to the back, and a cache configured to evict
/// drops the least recently used entry when it is full.
pub struct AbstractMapRetryContextCache<V> {
    capacity: AtomicUsize,
    fail_if_full: bool,
    map: Mutex<IndexMap<String, V>>,
    to_value: ToValue<V>,
    from_value: FromValue<V>,
}

impl<V> AbstractMapRetryContextCache<V>
where
    V: Clone + Send + Sync + 'static,
{
    /// Creates a cache.
    ///
    /// When `remove_eldest_entries` is `true` the least recently used entry is evicted at
    /// capacity; otherwise a full cache rejects new entries with
    /// [`RetryError::RetryCacheCapacityExceededError`].
    pub fn new(
        capacity: usize,
        remove_eldest_entries: bool,
        to_value: ToValue<V>,
        from_value: FromValue<V>,
    ) -> Self {
        Self {
            capacity: AtomicUsize::new(capacity.max(1)),
            fail_if_full: !remove_eldest_entries,
            map: Mutex::new(IndexMap::with_capacity(capacity.max(1))),
            to_value,
            from_value,
        }
    }

    /// Returns the configured capacity.
    pub fn capacity(&self) -> usize {
        self.capacity.load(Ordering::Acquire)
    }

    /// Updates the capacity.
    ///
    /// Shrinking takes effect on the next insertion; existing entries are kept until they
    /// are evicted or removed.
    pub fn set_capacity(&self, capacity: usize) {
        self.capacity.store(capacity.max(1), Ordering::Release);
    }

    /// Returns the number of stored entries, including entries whose value is stale.
    pub fn len(&self) -> usize {
        self.map.lock().map(|map| map.len()).unwrap_or_default()
    }

    /// Returns whether the cache holds no entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns whether the cache rejects new entries at capacity.
    pub fn fails_if_full(&self) -> bool {
        self.fail_if_full
    }
}

impl<V> RetryContextCache for AbstractMapRetryContextCache<V>
where
    V: Clone + Send + Sync + 'static,
{
    fn get(&self, key: &str) -> Option<Arc<dyn RetryContext>> {
        let mut map = self.map.lock().ok()?;
        let value = map.get(key)?.clone();
        let context = (self.from_value)(&value);
        if context.is_some() {
            // Track recency: a used entry must not be the first evicted.
            if let Some(stored) = map.shift_remove(key) {
                map.insert(key.to_owned(), stored);
            }
        } else {
            map.shift_remove(key);
        }
        context
    }

    fn put(&self, key: &str, context: Arc<dyn RetryContext>) -> Result<(), RetryError> {
        let mut map = self.map.lock().map_err(|_| {
            RetryError::cache_capacity_exceeded("retry context cache lock is poisoned")
        })?;

        if self.fail_if_full && !map.contains_key(key) && map.len() >= self.capacity() {
            return Err(RetryError::cache_capacity_exceeded(
                "Cache capacity limit breached. Do you need to re-consider the implementation of the key generator, or the equality of the items that failed?",
            ));
        }

        map.shift_remove(key);
        map.insert(key.to_owned(), (self.to_value)(context));

        while map.len() > self.capacity() {
            let Some(eldest) = map.keys().next().cloned() else {
                break;
            };
            warn!(
                capacity = self.capacity(),
                key = %eldest,
                "Retry cache capacity limit breached, evicting the least recently used entry. Do you need to re-consider the implementation of the key generator?"
            );
            map.shift_remove(&eldest);
        }

        Ok(())
    }

    fn remove(&self, key: &str) {
        if let Ok(mut map) = self.map.lock() {
            map.shift_remove(key);
        }
    }

    fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }
}

impl<V> Clone for AbstractMapRetryContextCache<V>
where
    V: Clone + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            capacity: AtomicUsize::new(self.capacity()),
            fail_if_full: self.fail_if_full,
            map: Mutex::new(self.map.lock().map(|map| map.clone()).unwrap_or_default()),
            to_value: self.to_value.clone(),
            from_value: self.from_value.clone(),
        }
    }
}

/// Returns the default capacity of an in-memory cache.
pub fn default_capacity() -> usize {
    DEFAULT_CAPACITY
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::AbstractMapRetryContextCache;
    use crate::{
        context::retry_context_support::RetryContextSupport,
        policy::retry_context_cache::RetryContextCache, retry_context::RetryContext,
    };

    fn strong_cache(
        capacity: usize,
        remove_eldest_entries: bool,
    ) -> AbstractMapRetryContextCache<Arc<dyn RetryContext>> {
        AbstractMapRetryContextCache::new(
            capacity,
            remove_eldest_entries,
            Arc::new(|context: Arc<dyn RetryContext>| context),
            Arc::new(|value: &Arc<dyn RetryContext>| Some(value.clone())),
        )
    }

    #[test]
    fn a_cache_returns_the_same_context_object() {
        let cache = strong_cache(4, true);
        let context: Arc<dyn RetryContext> = Arc::new(RetryContextSupport::new());

        cache
            .put("item", context.clone())
            .expect("capacity is free");
        let cached = cache.get("item").expect("context should exist");

        assert!(Arc::ptr_eq(&context, &cached));
        assert!(cache.contains_key("item"));
    }

    #[test]
    fn an_evicting_cache_drops_the_least_recently_used_entry() {
        let cache = strong_cache(2, true);
        let first: Arc<dyn RetryContext> = Arc::new(RetryContextSupport::new());
        let second: Arc<dyn RetryContext> = Arc::new(RetryContextSupport::new());
        let third: Arc<dyn RetryContext> = Arc::new(RetryContextSupport::new());

        cache.put("first", first).expect("capacity is free");
        cache.put("second", second).expect("capacity is free");
        // Touch the first entry so the second becomes the least recently used one.
        assert!(cache.get("first").is_some());
        cache.put("third", third).expect("eviction keeps room");

        assert!(cache.contains_key("first"));
        assert!(!cache.contains_key("second"));
        assert!(cache.contains_key("third"));
    }

    #[test]
    fn a_failing_cache_rejects_new_entries_at_capacity() {
        let cache = strong_cache(1, false);
        let first: Arc<dyn RetryContext> = Arc::new(RetryContextSupport::new());
        let second: Arc<dyn RetryContext> = Arc::new(RetryContextSupport::new());

        cache.put("first", first).expect("capacity is free");
        let error = cache
            .put("second", second)
            .expect_err("cache should reject the entry");

        assert!(matches!(
            error,
            crate::error::retry_error::RetryError::RetryCacheCapacityExceededError(_)
        ));
        assert_eq!(cache.len(), 1);
    }
}
