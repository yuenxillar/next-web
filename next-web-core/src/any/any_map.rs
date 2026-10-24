use std::{borrow::Borrow, collections::HashMap, hash::Hash, sync::Arc};

use dashmap::DashMap;
use tokio::sync::RwLock;

use crate::any::AnyValue;

/// A small concurrent map for storing values behind an async lock.
#[derive(Clone, Default)]
pub struct AnyMap<K = String, V = AnyValue> {
    /// The underlying map storing items.
    data: Arc<RwLock<HashMap<K, V>>>,
}

impl<K, V> AnyMap<K, V>
where
    K: Hash + Eq,
{
    /// Creates a new empty `AnyMap`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use next_web_core::any::AnyMap;
    /// let cache: AnyMap = AnyMap::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Inserts a key-value pair.
    ///
    /// # Arguments
    ///
    /// * `key` - Map key.
    /// * `value` - Value to store.
    ///
    /// # Examples
    ///
    /// ```
    /// # use next_web_core::any::AnyMap;
    /// # async fn example() {
    /// let cache = AnyMap::new();
    /// cache.insert("my_key".to_string(), "my_value".to_string()).await;
    /// # }
    /// ```
    pub async fn insert(&self, key: K, value: V) {
        self.data.write().await.insert(key, value);
    }

    /// Removes a value by key.
    pub async fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.data.write().await.remove(key)
    }

    /// Returns `true` when the key is present.
    pub async fn exists<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.data.read().await.contains_key(key)
    }

    /// Removes all entries.
    pub async fn clear(&self) {
        self.data.write().await.clear();
    }

    /// Returns the number of entries.
    #[must_use]
    pub async fn len(&self) -> usize {
        self.data.read().await.len()
    }

    /// Returns `true` when the map contains no entries.
    #[must_use]
    pub async fn is_empty(&self) -> bool {
        self.data.read().await.is_empty()
    }

    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(&K, &V),
    {
        self.data
            .try_read()
            .map(|map| map.iter().for_each(|(k, v)| f(k, v)))
            .ok();
    }
}

impl<K, V> AnyMap<K, V>
where
    K: Hash + Eq,
    V: Clone,
{
    /// Gets a cloned value by key.
    #[must_use]
    pub async fn get<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.data.read().await.get(key).cloned()
    }
}

impl<K, V> AnyMap<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    /// Extends this map with a cloned snapshot of another map.
    pub async fn extend(&self, other: Self) {
        if Arc::ptr_eq(&self.data, &other.data) {
            return;
        }

        let entries = other.data.read().await.clone();
        self.data.write().await.extend(entries);
    }

    #[must_use]
    pub async fn filter<F>(&self, f: F) -> HashMap<K, V>
    where
        F: Fn(&K, &V) -> bool,
    {
        self.data
            .read()
            .await
            .iter()
            .filter(|(k, v)| f(k, v))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

impl<K, V> AnyMap<K, V>
where
    K: ToString,
{
    #[must_use]
    pub async fn keys(&self) -> Vec<String> {
        self.data
            .read()
            .await
            .keys()
            .map(|k| k.to_string())
            .collect()
    }
}

/// A non-async, sharded concurrent map backed by [`DashMap`].
///
/// This is the synchronous counterpart to [`AnyMap`]. Instead of guarding a
/// single [`HashMap`] with one [`RwLock`], it uses the sharded locking scheme
/// from [`dashmap`], so independent keys are read and written without
/// contending and without any `.await`.
///
/// Reach for `AnyDashMap` when the map is touched from synchronous code, from
/// `spawn_blocking` work, or from a hot path that should not pay for an async
/// lock. Every clone shares the same underlying storage, so cloning is cheap
/// and mutations are visible through all handles.
///
/// # Examples
///
/// ```
/// use next_web_core::any::AnyDashMap;
///
/// let map: AnyDashMap<String, i64> = AnyDashMap::new();
/// map.insert("answer".to_string(), 42);
///
/// assert_eq!(map.get("answer"), Some(42));
/// ```
pub struct AnyDashMap<K = String, V = AnyValue> {
    /// The shared map. `Arc` makes cloning a handle cheap and lets all clones
    /// observe the same entries.
    data: Arc<DashMap<K, V>>,
}

impl<K, V> AnyDashMap<K, V>
where
    K: Eq + Hash,
{
    /// Creates an empty map.
    ///
    /// # Examples
    ///
    /// ```
    /// use next_web_core::any::AnyDashMap;
    ///
    /// let cache: AnyDashMap = AnyDashMap::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Creates an empty map pre-allocated for at least `capacity` entries.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Arc::new(DashMap::with_capacity(capacity)),
        }
    }

    /// Inserts a key-value pair, returning the previous value when the key
    /// already existed.
    ///
    /// # Examples
    ///
    /// ```
    /// use next_web_core::any::AnyDashMap;
    ///
    /// let cache: AnyDashMap<String, i32> = AnyDashMap::new();
    /// assert_eq!(cache.insert("k".to_string(), 1), None);
    /// assert_eq!(cache.insert("k".to_string(), 2), Some(1));
    /// ```
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        self.data.insert(key, value)
    }

    /// Removes `key` and returns its value when present.
    pub fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.data.remove(key).map(|(_, value)| value)
    }

    /// Returns `true` when `key` is present.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.data.contains_key(key)
    }

    /// Alias for [`AnyDashMap::contains_key`], kept for parity with
    /// [`AnyMap::exists`].
    pub fn exists<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.contains_key(key)
    }

    /// Removes every entry.
    pub fn clear(&self) {
        self.data.clear();
    }

    /// Returns the number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` when the map holds no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Calls `f` for every entry.
    ///
    /// A shard lock is held while its entries are visited, so `f` must not
    /// re-enter the map for a key that lives in the same shard.
    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(&K, &V),
    {
        for entry in self.data.iter() {
            f(entry.key(), entry.value());
        }
    }

    /// Keeps only the entries for which `f` returns `true`.
    pub fn retain<F>(&self, mut f: F)
    where
        F: FnMut(&K, &V) -> bool,
    {
        self.data.retain(|key, value| f(key, value));
    }
}

impl<K, V> AnyDashMap<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    /// Returns a clone of the value stored under `key`.
    ///
    /// Prefer [`AnyDashMap::get_with`] when cloning the value is expensive.
    #[must_use]
    pub fn get<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.data.get(key).map(|entry| entry.value().clone())
    }

    /// Borrows the value stored under `key` and maps it with `f` while the
    /// shard lock is held.
    ///
    /// This avoids cloning and is the cheapest read path.
    #[must_use]
    pub fn get_with<Q, R>(&self, key: &Q, f: impl FnOnce(&V) -> R) -> Option<R>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.data.get(key).map(|entry| f(entry.value()))
    }
}

impl<K, V> AnyDashMap<K, V>
where
    K: Eq + Hash + ToString,
{
    /// Returns a snapshot of every key rendered as a `String`.
    #[must_use]
    pub fn keys(&self) -> Vec<String> {
        self.data
            .iter()
            .map(|entry| entry.key().to_string())
            .collect()
    }
}

impl<K, V> Clone for AnyDashMap<K, V> {
    /// Clones only the `Arc` handle; the entries are shared, not copied.
    fn clone(&self) -> Self {
        Self {
            data: Arc::clone(&self.data),
        }
    }
}

impl<K, V> Default for AnyDashMap<K, V>
where
    K: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{AnyDashMap, AnyMap};

    #[test]
    fn dash_map_exposes_non_async_api() {
        let map = AnyDashMap::<String, i32>::new();

        assert_eq!(map.insert("answer".to_string(), 42), None);
        assert!(map.contains_key("answer"));
        assert!(map.exists("answer"));
        assert_eq!(map.get("answer"), Some(42));
        assert_eq!(map.get_with("answer", |value| *value + 1), Some(43));
        assert_eq!(map.remove("answer"), Some(42));
        assert!(!map.contains_key("answer"));
        assert!(map.is_empty());
    }

    #[test]
    fn dash_map_clones_share_storage() {
        let map = AnyDashMap::<String, i32>::new();
        let clone = map.clone();

        clone.insert("a".to_string(), 1);

        assert_eq!(map.get("a"), Some(1));
        assert_eq!(clone.len(), 1);
    }

    #[tokio::test]
    async fn borrowed_key_lookup_remove_and_exists_work() {
        let map = AnyMap::<String, i32>::new();
        map.insert("answer".to_string(), 42).await;

        assert!(map.exists("answer").await);
        assert_eq!(map.get("answer").await, Some(42));
        assert_eq!(map.remove("answer").await, Some(42));
        assert!(!map.exists("answer").await);
    }

    #[tokio::test]
    async fn extend_merges_distinct_maps() {
        let first = AnyMap::<String, i32>::new();
        let second = AnyMap::<String, i32>::new();

        first.insert("a".to_string(), 1).await;
        second.insert("b".to_string(), 2).await;

        first.extend(second).await;

        assert_eq!(first.get("a").await, Some(1));
        assert_eq!(first.get("b").await, Some(2));
    }

    #[tokio::test]
    async fn extend_with_clone_of_self_is_noop() {
        let map = AnyMap::<String, i32>::new();
        map.insert("a".to_string(), 1).await;

        map.extend(map.clone()).await;

        assert_eq!(map.len().await, 1);
        assert_eq!(map.get("a").await, Some(1));
    }
}
