use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::hash::Hash;
use std::sync::Arc;

/// Utility struct which wraps `Map` and notifies
/// `MapChangeListener` of changes for individual
/// change operations.
pub struct ObservableMap<K, V> {
    pub(super) delegate: HashMap<K, V>,
    pub(super) listener: Option<Arc<dyn MapChangeListener<K, V>>>,
}

impl<K, V> ObservableMap<K, V>
where
    K: Hash + Eq,
    K: 'static,
    V: 'static,
{
    pub fn new(delegate: HashMap<K, V>) -> Self {
        Self {
            delegate,
            listener: None,
        }
    }

    /// Instantiates a new observable map with existing map and listener.
    pub fn with_listener(
        delegate: HashMap<K, V>,
        listener: Arc<dyn MapChangeListener<K, V>>,
    ) -> Self {
        Self {
            delegate,
            listener: Some(listener),
        }
    }

    /// Gets the number of elements in the map.
    pub fn size(&self) -> usize {
        self.delegate.len()
    }

    /// Returns true if the map contains no elements.
    pub fn is_empty(&self) -> bool {
        self.delegate.is_empty()
    }

    /// Returns true if the map contains a value for the specified key.
    pub fn contains_key(&self, key: &K) -> bool {
        self.delegate.contains_key(key)
    }

    /// Returns the value to which the specified key is mapped.
    pub fn get(&self, key: &K) -> Option<&V> {
        self.delegate.get(key)
    }

    /// Removes the mapping for a key from the map if present.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let removed = self.delegate.remove(key);

        if let (Some(listener), Some(value)) = (&self.listener, &removed) {
            listener.removed(key, value);
        }

        removed
    }

    /// Copies all mappings from the specified map to this map.
    pub fn put_all(&mut self, map: HashMap<K, V>) {
        for (key, value) in map {
            self.delegate.insert(key, value);
        }
    }

    /// Removes all mappings from this map.
    pub fn clear(&mut self) {
        self.delegate.clear();
    }

    /// Returns a set view of the keys contained in this map.
    pub fn keys(&self) -> Vec<&K> {
        self.delegate.keys().collect()
    }

    /// Returns a collection view of the values contained in this map.
    pub fn values(&self) -> Vec<&V> {
        self.delegate.values().collect()
    }

    /// Returns a set view of the mappings contained in this map.
    pub fn entries(&self) -> Vec<(&K, &V)> {
        self.delegate.iter().map(|(k, v)| (k, v)).collect()
    }

    /// Gets the delegating map instance.
    pub fn get_delegate(&self) -> &HashMap<K, V> {
        &self.delegate
    }

    /// Sets the delegate map.
    pub fn set_delegate(&mut self, map: HashMap<K, V>) {
        self.delegate = map;
    }

    /// Sets the map change listener.
    pub fn set_listener(&mut self, listener: Arc<dyn MapChangeListener<K, V>>) {
        self.listener = Some(listener);
    }
}

impl<K, V> fmt::Display for ObservableMap<K, V>
where
    K: Debug,
    V: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format!("{:?}", self.delegate))
    }
}

impl<K, V> ObservableMap<K, V>
where
    K: 'static,
    K: Eq + Hash + Clone,
    V: Clone + PartialEq,
    V: 'static,
{
    /// Associates the specified value with the specified key in the map.
    pub fn put(&mut self, key: K, value: V) -> Option<V> {
        let old_value = self.delegate.insert(key.clone(), value.clone());

        if let Some(listener) = self.listener.as_ref() {
            if old_value.is_none() {
                listener.added(key, value);
            } else if let Some(old) = old_value.as_ref() {
                if &value != old {
                    listener.changed(&key, &value);
                }
            }
        }

        old_value
    }
}

impl<K, V> ObservableMap<K, V>
where
    K: Eq + Hash,
    V: PartialEq,
{
    /// Returns true if the map maps one or more keys to the specified value.
    pub fn contains_value(&self, value: &V) -> bool {
        self.delegate.values().any(|v| v == value)
    }
}
impl<K, V> fmt::Debug for ObservableMap<K, V>
where
    K: Eq + Hash + Clone + Send + Sync + fmt::Debug + 'static,
    V: Clone + Send + Sync + fmt::Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.delegate)
    }
}

impl<K, V> Default for ObservableMap<K, V> {
    fn default() -> Self {
        Self {
            delegate: HashMap::default(),
            listener: None,
        }
    }
}

/// Listener for map changes.
pub trait MapChangeListener<K, V>
where
    Self: Send + Sync,
    Self: 'static,
{
    /// Called when a key-value pair is added.
    fn added(&self, key: K, value: V);

    /// Called when a key-value pair is changed.
    fn changed(&self, key: &K, value: &V);

    /// Called when a key-value pair is removed.
    fn removed(&self, key: &K, value: &V);
}
