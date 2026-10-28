use std::{collections::HashMap, fmt::Display};

use crate::core::{
    cache::cache_manager::CacheManager,
    event::{event_bus_aware::EventBusAware, support::default_event_bus::DefaultEventBus},
    util::object::Object,
};

/// The cache manager of an application that keeps its caches in memory.
///
/// The manager holds one cache per name. A cache is created when the manager is
/// given it, or through [`DefaultCacheManager::get_or_create_cache`], because a
/// cache handed out by [`CacheManager::get_cache`] is borrowed from the manager.
#[derive(Clone, Default)]
pub struct DefaultCacheManager {
    caches: HashMap<String, HashMap<String, Object>>,
    event_bus: Option<DefaultEventBus>,
}

impl DefaultCacheManager {
    /// Creates a manager without a cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a cache, replacing the cache of the same name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the cache.
    /// * `cache` - The entries of the cache.
    pub fn add_cache(
        &mut self,
        name: impl ToString,
        cache: HashMap<String, Object>,
    ) -> Option<HashMap<String, Object>> {
        self.caches.insert(name.to_string(), cache)
    }

    /// Returns the cache of the given name, creating it when it does not exist.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the cache.
    pub fn get_or_create_cache(&mut self, name: &str) -> &mut HashMap<String, Object> {
        self.caches.entry(name.to_string()).or_default()
    }

    /// Returns the names of the caches of the manager.
    pub fn cache_names(&self) -> Vec<&str> {
        self.caches.keys().map(String::as_str).collect()
    }

    /// Removes a cache and returns its entries.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the cache.
    pub fn remove_cache(&mut self, name: &str) -> Option<HashMap<String, Object>> {
        self.caches.remove(name)
    }

    /// Returns the event bus the manager publishes with, when it has one.
    pub fn event_bus(&self) -> Option<&DefaultEventBus> {
        self.event_bus.as_ref()
    }
}

impl CacheManager for DefaultCacheManager {
    fn get_cache(&self, name: &str) -> Option<&HashMap<String, Object>> {
        self.caches.get(name)
    }
}

impl EventBusAware<DefaultEventBus> for DefaultCacheManager {
    fn set_event_bus(&mut self, event_bus: DefaultEventBus) {
        self.event_bus = Some(event_bus);
    }
}

impl Display for DefaultCacheManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DefaultCacheManager")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::DefaultCacheManager;
    use crate::core::{cache::cache_manager::CacheManager, util::object::Object};

    #[test]
    fn a_cache_is_readable_by_name() {
        let mut manager = DefaultCacheManager::new();
        manager.add_cache(
            "authenticationCache",
            HashMap::from([("root".to_string(), Object::Str("info".to_string()))]),
        );

        let cache = manager
            .get_cache("authenticationCache")
            .expect("the cache exists");
        assert_eq!(
            cache.get("root").and_then(Object::as_str),
            Some("info")
        );
        assert!(manager.get_cache("missing").is_none());
    }

    #[test]
    fn a_cache_is_created_when_it_is_asked_for() {
        let mut manager = DefaultCacheManager::default();

        manager
            .get_or_create_cache("authorizationCache")
            .insert("root".to_string(), Object::Bool(true));

        assert_eq!(manager.cache_names(), vec!["authorizationCache"]);
        assert!(manager.get_cache("authorizationCache").is_some());
    }

    #[test]
    fn a_cache_can_be_replaced_and_removed() {
        let mut manager = DefaultCacheManager::new();
        let first = manager.add_cache("cache", HashMap::new());
        assert!(first.is_none());

        let previous = manager.add_cache(
            "cache",
            HashMap::from([("key".to_string(), Object::Int(1))]),
        );
        assert!(previous.is_some());
        assert_eq!(
            manager.remove_cache("cache").map(|cache| cache.len()),
            Some(1)
        );
        assert!(manager.get_cache("cache").is_none());
    }
}
