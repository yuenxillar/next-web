use std::{
    any::{Any, TypeId, type_name},
    borrow::Cow,
    cmp::Ordering,
    collections::{HashMap, hash_map::Keys},
    hash::{Hash, Hasher},
    sync::Arc,
};

use crate::factory::config::SingletonRegistry;

/// Function that produces an owned copy of a type erased instance.
///
/// The function is shared (instead of being a bare `fn` pointer) because a
/// context may need to capture state in it, for example the provider that
/// recorded the clone function of the instance.
pub type InstanceClone =
    Arc<dyn Fn(&(dyn Any + Send + Sync)) -> Box<dyn Any + Send + Sync> + Send + Sync>;

pub struct DefaultSingletonRegistry {
    registry: HashMap<Key, DynSingle>,
}

impl DefaultSingletonRegistry {
    pub(crate) fn inner(&self) -> &HashMap<Key, DynSingle> {
        &self.registry
    }

    pub(crate) fn inner_mut(&mut self) -> &mut HashMap<Key, DynSingle> {
        &mut self.registry
    }

    pub(crate) fn insert(&mut self, key: Key, single: DynSingle) {
        // There is no need to check the value of `allow_override` here,
        // because when inserting a provider and a single with the same key into the context,
        // the provider must be inserted first, followed by the single,
        // and the checking of `allow_override` has already been done when the provider is inserted.
        self.registry.insert(key, single);
    }

    pub(crate) fn get_owned<T: 'static>(&self, key: &Key) -> Option<T> {
        self.registry.get(key)?.get_owned::<T>()
    }

    pub(crate) fn get_ref<T: 'static>(&self, key: &Key) -> Option<&T> {
        self.registry.get(key)?.as_ref::<T>()
    }

    pub(crate) fn get_mut<T: 'static>(&mut self, key: &Key) -> Option<&mut T> {
        self.registry.get_mut(key)?.as_mut::<T>()
    }

    pub(crate) fn contains(&self, key: &Key) -> bool {
        self.registry.contains_key(key)
    }

    pub(crate) fn remove(&mut self, key: &Key) -> Option<DynSingle> {
        self.registry.remove(key)
    }

    pub(crate) fn clear(&mut self) {
        self.registry.clear();
    }

    #[allow(unused)]
    pub(crate) fn keys<'a>(&'a self) -> Keys<'a, Key, DynSingle> {
        self.registry.keys()
    }

    pub fn is_empty(&self) -> bool {
        self.registry.is_empty()
    }

    /// Inserts an already type erased instance under the given key.
    ///
    /// This is the entry point for a context that stores its instances behind
    /// a [`Key`] and therefore cannot name their concrete type.
    ///
    /// # Arguments
    ///
    /// * `key` - The key identifying the instance.
    /// * `single` - The type erased instance.
    pub fn insert_dyn(&mut self, key: Key, single: DynSingle) {
        self.insert(key, single);
    }

    /// Returns whether an instance is registered under the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key identifying the instance.
    pub fn contains_key(&self, key: &Key) -> bool {
        self.contains(key)
    }

    /// Returns the type erased entry registered under the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key identifying the instance.
    pub fn get_dyn(&self, key: &Key) -> Option<&DynSingle> {
        self.registry.get(key)
    }

    /// Returns the type erased entry registered under the given key, mutably.
    ///
    /// # Arguments
    ///
    /// * `key` - The key identifying the instance.
    pub fn get_dyn_mut(&mut self, key: &Key) -> Option<&mut DynSingle> {
        self.registry.get_mut(key)
    }

    /// Removes and returns the type erased entry registered under the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key identifying the instance.
    pub fn remove_dyn(&mut self, key: &Key) -> Option<DynSingle> {
        self.remove(key)
    }

    /// Returns every type erased entry whose instance has the given type.
    ///
    /// # Arguments
    ///
    /// * `ty` - The [`TypeId`] of the instances to return.
    pub fn dyn_of_type(&self, ty: TypeId) -> Vec<&DynSingle> {
        self.registry
            .iter()
            .filter(|(key, _)| key.ty.id == ty)
            .map(|(_, single)| single)
            .collect()
    }

    /// Returns every type erased entry whose instance has the given type, mutably.
    ///
    /// # Arguments
    ///
    /// * `ty` - The [`TypeId`] of the instances to return.
    pub fn dyn_of_type_mut(&mut self, ty: TypeId) -> Vec<&mut DynSingle> {
        self.registry
            .iter_mut()
            .filter(|(key, _)| key.ty.id == ty)
            .map(|(_, single)| single)
            .collect()
    }

    /// Returns owned copies of every instance of the given type that recorded a
    /// clone function.
    ///
    /// # Arguments
    ///
    /// * `ty` - The [`TypeId`] of the instances to copy.
    pub fn owned_of_type(&self, ty: TypeId) -> Vec<Box<dyn Any + Send + Sync>> {
        self.dyn_of_type(ty)
            .into_iter()
            .filter_map(DynSingle::get_owned_boxed)
            .collect()
    }

    /// Returns the keys of every instance whose type is the given type.
    ///
    /// # Arguments
    ///
    /// * `ty` - The [`TypeId`] of the instances.
    pub fn keys_of_type(&self, ty: TypeId) -> Vec<Key> {
        self.registry
            .keys()
            .filter(|key| key.ty.id == ty)
            .cloned()
            .collect()
    }
}

impl SingletonRegistry for DefaultSingletonRegistry {
    fn register_singleton<T, N>(&mut self, name: N, singleton: T)
    where
        T: Send + Sync + 'static,
        N: Into<Cow<'static, str>>,
    {
        self.insert(
            Key::new::<T>(name.into()),
            DynSingle::from_uncloneable(singleton),
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
        T: Send + Sync + 'static,
        N: Into<Cow<'static, str>>,
    {
        let key = Key::new::<T>(name.into());
        self.registry
            .entry(key)
            .or_insert_with(|| DynSingle::from_uncloneable(default))
            .as_mut::<T>()
            .expect("Value was just inserted as type T")
    }

    fn register_cloneable_singleton<T, N>(&mut self, name: N, singleton: T)
    where
        T: Clone + Send + Sync + 'static,
        N: Into<Cow<'static, str>>,
    {
        self.insert(
            Key::new::<T>(name.into()),
            DynSingle::from_cloneable(singleton),
        );
    }

    fn get_singleton_owned<T, N>(&self, name: N) -> Option<T>
    where
        T: 'static,
        N: Into<Cow<'static, str>>,
    {
        let key = Key::new::<T>(name.into());
        self.get_owned(&key)
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
    /// Creates the key of a singleton of type `T` with the given name.
    ///
    /// A key identifies a stored instance by its type and its name, which is
    /// the pair a [`SingletonRegistry`] looks an instance up with.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the singleton.
    pub fn new<T: 'static>(name: Cow<'static, str>) -> Self {
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

/// Represents a [`Singleton`](crate::Scope::Singleton) or
/// [`SingleOwner`](crate::Scope::SingleOwner) instance whose type was erased.
///
/// The instance is stored as a trait object so that a registry can hold values
/// of any type, and can be inserted either from a concrete value (which records
/// whether the value can be cloned) or from an already boxed instance. The
/// latter is what lets an application context insert the instances it receives
/// from its own, type erased API.
pub struct DynSingle {
    /// The erased singleton.
    ///
    /// The `Send + Sync` bounds are part of the type so that a
    /// [`DefaultSingletonRegistry`] can be stored inside a shareable
    /// application context. Registration therefore requires `T: Send + Sync`.
    instance: Box<dyn Any + Send + Sync>,
    /// Produces an owned copy of the stored instance when one was recorded.
    clone: Option<InstanceClone>,
}

impl DynSingle {
    /// Wraps an already boxed instance.
    ///
    /// # Arguments
    ///
    /// * `instance` - The boxed instance.
    /// * `clone` - The function producing owned copies of the instance, when the
    ///   instance may be handed out as an owned value.
    pub fn new(
        instance: Box<dyn Any + Send + Sync>,
        clone: Option<InstanceClone>,
    ) -> Self {
        Self { instance, clone }
    }

    /// Wraps a value that can be cloned.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    ///
    /// # Arguments
    ///
    /// * `instance` - The instance to wrap.
    pub fn from_cloneable<T>(instance: T) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        Self::new(Box::new(instance), Some(Arc::new(clone_boxed::<T>)))
    }

    /// Wraps a value that only supports being borrowed.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the instance.
    ///
    /// # Arguments
    ///
    /// * `instance` - The instance to wrap.
    pub fn from_uncloneable<T>(instance: T) -> Self
    where
        T: Send + Sync + 'static,
    {
        Self::new(Box::new(instance), None)
    }

    /// Returns the stored instance as a type erased reference.
    pub fn as_any(&self) -> &(dyn Any + Send + Sync) {
        &*self.instance
    }

    /// Returns the stored instance as a type erased mutable reference.
    pub fn as_any_mut(&mut self) -> &mut (dyn Any + Send + Sync) {
        &mut *self.instance
    }

    /// Returns an owned copy of the stored instance.
    ///
    /// Returns `None` when the instance was registered without a clone
    /// function, in which case only borrows of it can be handed out.
    pub fn get_owned_boxed(&self) -> Option<Box<dyn Any + Send + Sync>> {
        Some((self.clone.as_ref())?(self.as_any()))
    }

    /// Returns a reference to the instance when it has the given type.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The expected type of the instance.
    pub fn as_ref<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.instance.downcast_ref::<T>()
    }

    /// Returns a mutable reference to the instance when it has the given type.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The expected type of the instance.
    pub fn as_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static,
    {
        self.instance.downcast_mut::<T>()
    }

    /// Returns an owned copy of the instance when it has the given type.
    ///
    /// Returns `None` when the instance cannot be cloned.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The expected type of the instance.
    pub fn get_owned<T>(&self) -> Option<T>
    where
        T: 'static,
    {
        self.get_owned_boxed()?.downcast::<T>().ok().map(|value| *value)
    }

    /// Consumes this [`DynSingle`] and returns the instance when it has the
    /// given type.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The expected type of the instance.
    pub fn into_inner<T>(self) -> Option<T>
    where
        T: 'static,
    {
        self.instance.downcast::<T>().ok().map(|value| *value)
    }

    /// Consumes this [`DynSingle`] and returns the type erased instance.
    pub fn into_inner_boxed(self) -> Box<dyn Any + Send + Sync> {
        self.instance
    }
}

/// Clones the instance held by a type erased reference.
///
/// # Type Parameters
///
/// * `T` - The concrete type of the instance.
///
/// # Arguments
///
/// * `instance` - The type erased instance to copy.
fn clone_boxed<T>(instance: &(dyn Any + Send + Sync)) -> Box<dyn Any + Send + Sync>
where
    T: Clone + Send + Sync + 'static,
{
    let instance = instance
        .downcast_ref::<T>()
        .expect("the erased instance holds its own type");

    Box::new(instance.clone())
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

    // ========== DynSingle Tests ==========

    #[test]
    fn test_dyn_single_from_cloneable_is_cloneable() {
        let dyn_single = DynSingle::from_cloneable(DatabaseConfig::new("localhost:5432", 10));

        let downcasted = dyn_single.as_ref::<DatabaseConfig>();
        assert!(downcasted.is_some());
        assert_eq!(downcasted.unwrap().url, "localhost:5432");

        assert_eq!(
            dyn_single.get_owned::<DatabaseConfig>(),
            Some(DatabaseConfig::new("localhost:5432", 10))
        );
    }

    #[test]
    fn test_dyn_single_from_uncloneable_is_not_cloneable() {
        let dyn_single = DynSingle::from_uncloneable(DatabaseConfig::new("localhost:5432", 10));

        assert!(dyn_single.as_ref::<DatabaseConfig>().is_some());
        assert!(dyn_single.get_owned::<DatabaseConfig>().is_none());
    }

    #[test]
    fn test_dyn_single_downcast_wrong_type() {
        let dyn_single = DynSingle::from_uncloneable(DatabaseConfig::new("localhost:5432", 10));
        // 尝试用错误的类型 downcast
        let result = dyn_single.as_ref::<CacheConfig>();
        assert!(result.is_none());
    }

    #[test]
    fn test_dyn_single_as_mut() {
        let mut dyn_single = DynSingle::from_uncloneable(DatabaseConfig::new("localhost:5432", 10));

        let downcasted = dyn_single.as_mut::<DatabaseConfig>();
        assert!(downcasted.is_some());
        downcasted.unwrap().pool_size = 50;

        assert_eq!(dyn_single.as_ref::<DatabaseConfig>().unwrap().pool_size, 50);
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
    fn test_register_cloneable_singleton_hands_out_owned_copies() {
        let mut registry = DefaultSingletonRegistry::default();
        registry.register_cloneable_singleton("db", DatabaseConfig::new("localhost", 3));

        let owned = registry.get_singleton_owned::<DatabaseConfig, _>("db");
        assert_eq!(owned, Some(DatabaseConfig::new("localhost", 3)));

        // Producing a copy must not remove the stored instance.
        assert!(registry.contains_singleton::<DatabaseConfig, _>("db"));
        assert_eq!(
            registry
                .get_singleton::<DatabaseConfig, _>("db")
                .unwrap()
                .pool_size,
            3
        );
    }

    #[test]
    fn test_get_singleton_owned_requires_a_recorded_clone() {
        let mut registry = DefaultSingletonRegistry::default();
        registry.register_singleton("db", DatabaseConfig::new("localhost", 3));

        // `register_singleton` stores no clone function, so no owned copy exists.
        assert_eq!(registry.get_singleton_owned::<DatabaseConfig, _>("db"), None);
        assert!(registry.contains_singleton::<DatabaseConfig, _>("db"));
    }

    #[test]
    fn test_get_singleton_owned_of_a_missing_singleton() {
        let registry = DefaultSingletonRegistry::default();

        assert_eq!(registry.get_singleton_owned::<DatabaseConfig, _>("db"), None);
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
        let single = DynSingle::from_uncloneable(DatabaseConfig::default());

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
