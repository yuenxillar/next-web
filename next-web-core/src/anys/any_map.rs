use std::{any::Any, borrow::Borrow, collections::HashMap, hash::Hash, sync::Arc};

use tokio::sync::RwLock;

use crate::anys::any_value::AnyValue;

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
    /// # use next_web_core::anys::any_map::AnyMap;
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
    /// # use next_web_core::anys::any_map::AnyMap;
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

/// Cache object wrapper.
pub struct CacheObject<T>(pub T)
where
    T: Any + Clone + Send + Sync;

impl<T: 'static + Any + Clone + Send + Sync> From<CacheObject<T>> for AnyValue {
    fn from(value: CacheObject<T>) -> Self {
        AnyValue::Object(Box::new(value.0))
    }
}

#[cfg(test)]
mod tests {
    use super::AnyMap;

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
