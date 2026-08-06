use std::{
    any::{type_name, Any, TypeId},
    borrow::Cow,
    cmp::Ordering,
    collections::{hash_map::Keys, HashMap},
    hash::{Hash, Hasher},
};

use crate::factory::config::SingletonRegistry;

pub struct DefaultSingletonRegistry {
    registry: HashMap<Key, DynSingle>,
}

impl DefaultSingletonRegistry {
    pub(crate) fn inner(&self) -> &HashMap<Key, DynSingle> {
        &self.registry
    }

    pub(crate) fn insert(&mut self, key: Key, single: DynSingle) {
        // There is no need to check the value of `allow_override` here,
        // because when inserting a provider and a single with the same key into the context,
        // the provider must be inserted first, followed by the single,
        // and the checking of `allow_override` has already been done when the provider is inserted.
        self.registry.insert(key, single);
    }

    pub(crate) fn get_owned<T: 'static>(&self, key: &Key) -> Option<T> {
        self.registry.get(key)?.as_single::<T>()?.get_owned()
    }

    pub(crate) fn get_ref<T: 'static>(&self, key: &Key) -> Option<&T> {
        Some(self.registry.get(key)?.as_single::<T>()?.get_ref())
    }

    pub(crate) fn get_mut<T: 'static>(&mut self, key: &Key) -> Option<&mut T> {
        Some(self.registry.get_mut(key)?.as_single_mut::<T>()?.get_mut())
    }

    pub(crate) fn contains(&self, key: &Key) -> bool {
        self.registry.contains_key(key)
    }

    pub(crate) fn remove(&mut self, key: &Key) -> Option<DynSingle> {
        self.registry.remove(key)
    }

    #[allow(unused)]
    pub(crate) fn keys<'a>(&'a self) -> Keys<'a, Key, DynSingle> {
        self.registry.keys()
    }

    pub fn is_empty(&self) -> bool {
        self.registry.is_empty()
    }
}

impl SingletonRegistry for DefaultSingletonRegistry {
    fn register_singleton<T, N>(&mut self, name: N, singleton: T)
    where
        T: 'static,
        N: Into<Cow<'static, str>>,
    {
        self.insert(
            Key::new::<T>(name.into()),
            DynSingle::from(Single::new(singleton, None)),
        );
    }

    fn get_singleton<T, N>(&mut self, name: N) -> Option<&T>
    where
        T: 'static,
        N: Into<Cow<'static, str>>,
    {
        let key = Key::new::<T>(name.into());
        self.get_ref(&key)
    }

    fn get_singleton_mut<T, N>(&mut self, name: N) -> Option<&mut T>
    where
        T: 'static,
        N: Into<Cow<'static, str>>,
    {
        let key = Key::new::<T>(name.into());
        self.get_mut(&key)
    }

    fn contains_singleton<T, N>(&self, name: N) -> bool
    where
        T: 'static,
        N: Into<Cow<'static, str>>,
    {
        let key = Key::new::<T>(name.into());
        self.contains(&key)
    }

    fn get_singleton_or_insert<T, N>(&mut self, name: N, default: T) -> &mut T
    where
        T: 'static,
        N: Into<Cow<'static, str>>,
    {
        let key = Key::new::<T>(name.into());
        self.registry
            .entry(key)
            .or_insert_with(|| DynSingle::from(Single::new(default, None)))
            .as_single_mut()
            .map(Single::get_mut)
            .expect("Value was just inserted as type T")
    }

    fn get_singleton_names<T>(&self) -> impl Iterator<Item = &Cow<'static, str>>
    where
        T: 'static,
    {
        self.registry
            .iter()
            .filter(|(k, _)| k.ty == Type::new::<T>())
            .map(|(k, _)| &k.name)
    }

    fn get_all_singleton_names(&self) -> impl Iterator<Item = &Cow<'static, str>> {
        self.keys().map(|k| &k.name)
    }

    fn get_singleton_count(&self) -> usize {
        self.registry.len()
    }
}

impl Default for DefaultSingletonRegistry {
    fn default() -> Self {
        Self {
            registry: HashMap::with_capacity(256),
        }
    }
}

/// Represents a unique key for a provider.
#[derive(Clone, Debug)]
pub struct Key {
    /// The name of the provider.
    pub name: Cow<'static, str>,
    /// The type of the provider generic.
    pub ty: Type,
}

impl Key {
    pub(crate) fn new<T: 'static>(name: Cow<'static, str>) -> Self {
        Self {
            name,
            ty: Type::new::<T>(),
        }
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.ty == other.ty && self.name == other.name
    }
}

impl Eq for Key {}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.ty.cmp(&other.ty) {
            Ordering::Equal => {}
            ord => return ord,
        }
        self.name.cmp(&other.name)
    }
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ty.hash(state);
        self.name.hash(state);
    }
}

/// Represents a type.
#[derive(Clone, Copy, Debug)]
pub struct Type {
    /// The name of the type.
    pub name: &'static str,
    /// The unique identifier of the type.
    pub id: TypeId,
}

impl Type {
    pub fn new<T: 'static>() -> Type {
        Type {
            name: type_name::<T>(),
            id: TypeId::of::<T>(),
        }
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Type {}

impl PartialOrd for Type {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Type {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl Hash for Type {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// Represents a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance.
pub struct Single<T> {
    instance: T,
    clone: Option<fn(&T) -> T>,
}

impl<T> Single<T> {
    pub(crate) fn new(instance: T, clone: Option<fn(&T) -> T>) -> Self {
        Self { instance, clone }
    }

    /// Returns the owned instance.
    pub fn get_owned(&self) -> Option<T> {
        self.clone.map(|clone| clone(&self.instance))
    }

    /// Returns a reference to the instance.
    pub fn get_ref(&self) -> &T {
        &self.instance
    }

    /// Returns a mutable reference to the instance.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.instance
    }
}

/// Represents a [`Single`] that erased its type.
pub struct DynSingle {
    origin: Box<dyn Any>,
}

impl DynSingle {
    /// Returns a reference of the origin [`Single`].
    pub fn as_single<T>(&self) -> Option<&Single<T>>
    where
        T: 'static,
    {
        self.origin.downcast_ref::<Single<T>>()
    }

    /// Returns a mutable reference of the origin [`Single`].
    pub fn as_single_mut<T>(&mut self) -> Option<&mut Single<T>>
    where
        T: 'static,
    {
        self.origin.downcast_mut::<Single<T>>()
    }
}

impl<T> From<Single<T>> for DynSingle
where
    T: 'static,
{
    fn from(value: Single<T>) -> Self {
        Self {
            origin: Box::new(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    #[derive(Debug, PartialEq, Eq, Clone, Default)]
    struct DatabaseConfig {
        url: String,
        pool_size: u32,
    }

    impl DatabaseConfig {
        fn new(url: &str, pool_size: u32) -> Self {
            Self {
                url: url.to_string(),
                pool_size,
            }
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone, Default)]
    struct CacheConfig {
        max_size: usize,
        ttl_seconds: u64,
    }

    impl CacheConfig {
        fn new(max_size: usize, ttl_seconds: u64) -> Self {
            Self {
                max_size,
                ttl_seconds,
            }
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone, Default)]
    struct LoggerConfig {
        level: String,
        file: Option<String>,
    }

    // ========== Type Tests ==========

    #[test]
    fn test_type_new_same_type_equal() {
        let t1 = Type::new::<DatabaseConfig>();
        let t2 = Type::new::<DatabaseConfig>();
        assert_eq!(t1, t2);
        assert_eq!(t1.id, t2.id);
    }

    #[test]
    fn test_type_new_different_types_not_equal() {
        let t1 = Type::new::<DatabaseConfig>();
        let t2 = Type::new::<CacheConfig>();
        assert_ne!(t1, t2);
        assert_ne!(t1.id, t2.id);
    }

    #[test]
    fn test_type_hash_consistent() {
        use std::collections::hash_map::DefaultHasher;
        let t1 = Type::new::<DatabaseConfig>();
        let t2 = Type::new::<DatabaseConfig>();
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        t1.hash(&mut h1);
        t2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn test_type_name_contains_type_info() {
        let t = Type::new::<DatabaseConfig>();
        assert!(t.name.contains("DatabaseConfig"));
    }

    #[test]
    fn test_type_clone_copy() {
        let t1 = Type::new::<DatabaseConfig>();
        let t2 = t1; // Copy
        assert_eq!(t1, t2);
        let t3 = t1.clone(); // Clone
        assert_eq!(t1, t3);
    }

    // ========== Key Tests ==========

    #[test]
    fn test_key_new() {
        let key = Key::new::<DatabaseConfig>(Cow::Borrowed("main-db"));
        assert_eq!(key.name, "main-db");
        assert_eq!(key.ty, Type::new::<DatabaseConfig>());
    }

    #[test]
    fn test_key_equality_same_type_and_name() {
        let k1 = Key::new::<DatabaseConfig>(Cow::Borrowed("db"));
        let k2 = Key::new::<DatabaseConfig>(Cow::Borrowed("db"));
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_key_inequality_different_name() {
        let k1 = Key::new::<DatabaseConfig>(Cow::Borrowed("db1"));
        let k2 = Key::new::<DatabaseConfig>(Cow::Borrowed("db2"));
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_key_inequality_different_type() {
        let k1 = Key::new::<DatabaseConfig>(Cow::Borrowed("config"));
        let k2 = Key::new::<CacheConfig>(Cow::Borrowed("config"));
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_key_ordering() {
        let k1 = Key::new::<DatabaseConfig>(Cow::Borrowed("a"));
        let k2 = Key::new::<DatabaseConfig>(Cow::Borrowed("b"));
        assert!(k1 < k2);
    }

    #[test]
    fn test_key_hash_consistent() {
        use std::collections::hash_map::DefaultHasher;
        let k1 = Key::new::<DatabaseConfig>(Cow::Borrowed("db"));
        let k2 = Key::new::<DatabaseConfig>(Cow::Borrowed("db"));
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        k1.hash(&mut h1);
        k2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    // ========== Single Tests ==========

    #[test]
    fn test_single_new_without_clone() {
        let config = DatabaseConfig::new("localhost:5432", 10);
        let single = Single::new(config, None);
        assert_eq!(single.get_ref().url, "localhost:5432");
        assert_eq!(single.get_ref().pool_size, 10);
        assert!(single.get_owned().is_none());
    }

    #[test]
    fn test_single_new_with_clone() {
        let config = DatabaseConfig::new("localhost:5432", 10);
        let clone_fn: fn(&DatabaseConfig) -> DatabaseConfig = |c| c.clone();
        let single = Single::new(config, Some(clone_fn));
        let owned = single.get_owned();
        assert!(owned.is_some());
        assert_eq!(owned.unwrap().url, "localhost:5432");
    }

    #[test]
    fn test_single_get_mut() {
        let config = DatabaseConfig::new("localhost:5432", 10);
        let mut single = Single::new(config, None);
        single.get_mut().pool_size = 20;
        assert_eq!(single.get_ref().pool_size, 20);
    }

    // ========== DynSingle Tests ==========

    #[test]
    fn test_dyn_single_new_and_downcast() {
        let config = DatabaseConfig::new("localhost:5432", 10);
        let clone_fn: fn(&DatabaseConfig) -> DatabaseConfig = |c| c.clone();
        let single = Single::new(config, Some(clone_fn));
        let dyn_single = DynSingle::from(single);

        let downcasted = dyn_single.as_single::<DatabaseConfig>();
        assert!(downcasted.is_some());
        assert_eq!(downcasted.unwrap().get_ref().url, "localhost:5432");
    }

    #[test]
    fn test_dyn_single_downcast_wrong_type() {
        let config = DatabaseConfig::new("localhost:5432", 10);
        let single = Single::new(config, None);
        let dyn_single = DynSingle::from(single);
        // 尝试用错误的类型 downcast
        let result = dyn_single.as_single::<CacheConfig>();
        assert!(result.is_none());
    }

    #[test]
    fn test_dyn_single_as_single_mut() {
        let config = DatabaseConfig::new("localhost:5432", 10);
        let single = Single::new(config, None);
        let mut dyn_single = DynSingle::from(single);

        let downcasted = dyn_single.as_single_mut::<DatabaseConfig>();
        assert!(downcasted.is_some());
        downcasted.unwrap().get_mut().pool_size = 50;
    }

    // ========== DefaultSingletonRegistry Tests ==========

    #[test]
    fn test_registry_default_empty() {
        let registry = DefaultSingletonRegistry::default();
        assert!(registry.is_empty());
        assert_eq!(registry.get_singleton_count(), 0);
    }

    #[test]
    fn test_register_and_get_singleton() {
        let mut registry = DefaultSingletonRegistry::default();
        let config = DatabaseConfig::new("localhost:5432", 10);

        registry.register_singleton("main-db", config);

        let retrieved: &DatabaseConfig = registry.get_singleton("main-db").unwrap();
        assert_eq!(retrieved.url, "localhost:5432");
        assert_eq!(retrieved.pool_size, 10);
    }

    #[test]
    fn test_get_singleton_not_found() {
        let mut registry = DefaultSingletonRegistry::default();
        let result: Option<&DatabaseConfig> = registry.get_singleton("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_singleton_wrong_type() {
        let mut registry = DefaultSingletonRegistry::default();
        let config = DatabaseConfig::new("localhost:5432", 10);
        registry.register_singleton("config", config);

        let result: Option<&CacheConfig> = registry.get_singleton("config");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_singleton_mut() {
        let mut registry = DefaultSingletonRegistry::default();
        let config = DatabaseConfig::new("localhost:5432", 10);
        registry.register_singleton("main-db", config);

        let retrieved = registry
            .get_singleton_mut::<DatabaseConfig, _>("main-db")
            .unwrap();
        retrieved.pool_size = 100;

        let updated = registry
            .get_singleton::<DatabaseConfig, _>("main-db")
            .unwrap();
        assert_eq!(updated.pool_size, 100);
    }

    #[test]
    fn test_contains_singleton() {
        let mut registry = DefaultSingletonRegistry::default();
        assert!(!registry.contains_singleton::<DatabaseConfig, _>("main-db"));

        registry.register_singleton("main-db", DatabaseConfig::default());
        assert!(registry.contains_singleton::<DatabaseConfig, _>("main-db"));
    }

    #[test]
    fn test_get_singleton_or_insert() {
        let mut registry = DefaultSingletonRegistry::default();

        let config =
            registry.get_singleton_or_insert("main-db", DatabaseConfig::new("localhost", 5));
        assert_eq!(config.url, "localhost");
        assert_eq!(config.pool_size, 5);

        let config2 =
            registry.get_singleton_or_insert("main-db", DatabaseConfig::new("ignored", 999));
        assert_eq!(config2.url, "localhost");
        assert_eq!(config2.pool_size, 5);
    }

    #[test]
    fn test_get_singleton_or_insert_mutability() {
        let mut registry = DefaultSingletonRegistry::default();

        let config =
            registry.get_singleton_or_insert("main-db", DatabaseConfig::new("localhost", 5));
        config.pool_size = 50;

        let updated = registry
            .get_singleton::<DatabaseConfig, _>("main-db")
            .unwrap();
        assert_eq!(updated.pool_size, 50);
    }

    #[test]
    fn test_register_multiple_types() {
        let mut registry = DefaultSingletonRegistry::default();

        registry.register_singleton("db-config", DatabaseConfig::new("db-url", 10));
        registry.register_singleton("cache-config", CacheConfig::new(1024, 3600));
        registry.register_singleton(
            "logger",
            LoggerConfig {
                level: "debug".to_string(),
                file: Some("app.log".to_string()),
            },
        );

        assert_eq!(registry.get_singleton_count(), 3);

        assert!(registry.contains_singleton::<DatabaseConfig, _>("db-config"));
        assert!(registry.contains_singleton::<CacheConfig, _>("cache-config"));
        assert!(registry.contains_singleton::<LoggerConfig, _>("logger"));
    }

    #[test]
    fn test_same_name_different_types() {
        let mut registry = DefaultSingletonRegistry::default();

        registry.register_singleton("config", DatabaseConfig::new("db-url", 10));
        registry.register_singleton("config", CacheConfig::new(2048, 7200));

        let db: &DatabaseConfig = registry.get_singleton("config").unwrap();
        assert_eq!(db.pool_size, 10);

        let cache: &CacheConfig = registry.get_singleton("config").unwrap();
        assert_eq!(cache.max_size, 2048);

        assert_eq!(registry.get_singleton_count(), 2);
    }

    #[test]
    fn test_get_singleton_names() {
        let mut registry = DefaultSingletonRegistry::default();

        registry.register_singleton("db1", DatabaseConfig::new("url1", 1));
        registry.register_singleton("db2", DatabaseConfig::new("url2", 2));
        registry.register_singleton("cache1", CacheConfig::new(100, 60));

        let names: Vec<&Cow<'static, str>> =
            registry.get_singleton_names::<DatabaseConfig>().collect();
        assert_eq!(names.len(), 2);
        assert!(names.iter().any(|n| n.as_ref() == "db1"));
        assert!(names.iter().any(|n| n.as_ref() == "db2"));
        assert!(!names.iter().any(|n| n.as_ref() == "cache1"));
    }

    #[test]
    fn test_get_all_singleton_names() {
        let mut registry = DefaultSingletonRegistry::default();

        registry.register_singleton("db", DatabaseConfig::default());
        registry.register_singleton("cache", CacheConfig::default());
        registry.register_singleton("logger", LoggerConfig::default());

        let all_names: Vec<&Cow<'static, str>> = registry.get_all_singleton_names().collect();
        assert_eq!(all_names.len(), 3);
        assert!(all_names.iter().any(|n| n.as_ref() == "db"));
        assert!(all_names.iter().any(|n| n.as_ref() == "cache"));
        assert!(all_names.iter().any(|n| n.as_ref() == "logger"));
    }

    #[test]
    fn test_get_singleton_count() {
        let mut registry = DefaultSingletonRegistry::default();
        assert_eq!(registry.get_singleton_count(), 0);

        registry.register_singleton("a", DatabaseConfig::default());
        assert_eq!(registry.get_singleton_count(), 1);

        registry.register_singleton("b", CacheConfig::default());
        assert_eq!(registry.get_singleton_count(), 2);

        registry.register_singleton("a", DatabaseConfig::default());
        assert_eq!(registry.get_singleton_count(), 2);
    }

    #[test]
    fn test_register_overwrite() {
        let mut registry = DefaultSingletonRegistry::default();

        registry.register_singleton("config", DatabaseConfig::new("first", 1));
        registry.register_singleton("config", DatabaseConfig::new("second", 2));

        let config: &DatabaseConfig = registry.get_singleton("config").unwrap();
        assert_eq!(config.url, "second"); // 被覆盖了
        assert_eq!(registry.get_singleton_count(), 1); // 数量不变
    }

    #[test]
    fn test_is_empty() {
        let mut registry = DefaultSingletonRegistry::default();
        assert!(registry.is_empty());

        registry.register_singleton("test", DatabaseConfig::default());
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_inner_access() {
        let mut registry = DefaultSingletonRegistry::default();
        registry.register_singleton("test", DatabaseConfig::default());

        let inner = registry.inner();
        assert_eq!(inner.len(), 1);

        let key = Key::new::<DatabaseConfig>(Cow::Borrowed("test"));
        assert!(inner.contains_key(&key));
    }

    #[test]
    fn test_insert_and_remove() {
        let mut registry = DefaultSingletonRegistry::default();
        let key = Key::new::<DatabaseConfig>(Cow::Borrowed("test"));
        let single = DynSingle::from(Single::new(DatabaseConfig::default(), None));

        registry.insert(key.clone(), single);
        assert!(registry.contains(&key));
        assert_eq!(registry.get_singleton_count(), 1);

        let removed = registry.remove(&key);
        assert!(removed.is_some());
        assert!(!registry.contains(&key));
        assert_eq!(registry.get_singleton_count(), 0);
    }

    #[test]
    fn test_keys_iterator() {
        let mut registry = DefaultSingletonRegistry::default();
        registry.register_singleton("one", DatabaseConfig::default());
        registry.register_singleton("two", CacheConfig::default());

        let keys: Vec<&Key> = registry.keys().collect();
        assert_eq!(keys.len(), 2);

        let names: Vec<&str> = keys.iter().map(|k| k.name.as_ref()).collect();
        assert!(names.contains(&"one"));
        assert!(names.contains(&"two"));
    }
}
