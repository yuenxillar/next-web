use std::borrow::Cow;

use crate::factory::config::SingletonRegistry;

/// A factory trait for creating and managing application-level components.
///
/// Combines singleton management with lazy initialization, allowing components
/// to be created on-demand and shared across the application lifecycle.
pub trait SingletonFactory {
    /// The type of singleton registry used internally.
    type Registry: SingletonRegistry;

    /// Returns a reference to the underlying singleton registry.
    fn registry(&self) -> &Self::Registry;

    /// Returns a mutable reference to the underlying singleton registry.
    fn registry_mut(&mut self) -> &mut Self::Registry;

    /// Gets an existing singleton of type `T` with the given name, or creates one
    /// using the provided factory function if it doesn't exist.
    ///
    /// This is the primary lazy-initialization method. The factory function is only
    /// called when a singleton with the given name and type does not already exist.
    ///
    /// # Type Parameters
    /// - `T`: The type of singleton to retrieve or create. Must be `'static`.
    /// - `N`: The name type, which can be converted into a `Cow<'static, str>`.
    /// - `F`: The factory function type that produces `T`.
    ///
    /// # Examples
    /// ```ignore
    /// let config: &Config = factory.get_or_create("app-config", || Config::load());
    /// let db_pool: &DbPool = factory.get_or_create("db-pool", || DbPool::connect());
    /// ```
    fn get_or_create<T, N, F>(&mut self, name: N, factory: F) -> &T
    where
        T: 'static,
        N: Into<Cow<'static, str>>,
        F: FnOnce() -> T,
    {
        let name = name.into();
        if !self.registry().contains_singleton::<T, _>(name.clone()) {
            let instance = factory();
            self.registry_mut()
                .register_singleton(name.to_owned(), instance);
        }
        self.registry_mut()
            .get_singleton::<T, _>(name)
            .expect("Singleton was just registered")
    }

    /// Gets an existing singleton of type `T` with the given name, or creates one
    /// using `T::default()` if it doesn't exist.
    ///
    /// Convenience method that uses the `Default` trait instead of a custom factory.
    ///
    /// # Type Parameters
    /// - `T`: The type of singleton. Must implement `Default` and be `'static`.
    /// - `N`: The name type.
    fn get_or_default<T, N>(&mut self, name: N) -> &T
    where
        T: Default + 'static,
        N: Into<Cow<'static, str>>,
    {
        self.get_or_create(name, T::default)
    }

    /// Gets a mutable reference to an existing singleton, or creates one using
    /// the factory function if it doesn't exist.
    ///
    /// # Type Parameters
    /// - `T`: The type of singleton. Must be `'static`.
    /// - `N`: The name type.
    /// - `F`: The factory function type.
    fn get_or_create_mut<T, N, F>(&mut self, name: N, factory: F) -> &mut T
    where
        T: 'static,
        N: Into<Cow<'static, str>>,
        F: FnOnce() -> T,
    {
        let name: Cow<'static, str> = name.into();
        if !self.registry().contains_singleton::<T, _>(name.clone()) {
            let instance = factory();
            self.registry_mut()
                .register_singleton(name.clone(), instance);
        }
        self.registry_mut()
            .get_singleton_mut::<T, _>(name)
            .expect("Singleton was just registered")
    }

    /// Gets a mutable reference to an existing singleton, or creates one using
    /// `T::default()` if it doesn't exist.
    fn get_or_default_mut<T, N>(&mut self, name: N) -> &mut T
    where
        T: Default + 'static,
        N: Into<Cow<'static, str>>,
    {
        self.get_or_create_mut(name, T::default)
    }

    /// Creates and registers all essential application singletons that should be
    /// available at startup.
    ///
    /// Override this method in implementations to initialize core components
    /// like configuration, database pools, logging, etc.
    ///
    /// # Examples
    /// ```ignore
    /// fn initialize_defaults(&mut self) {
    ///     self.get_or_create("config", || AppConfig::from_env());
    ///     self.get_or_create("logger", || Logger::init());
    ///     self.get_or_create("db-pool", || DbPool::new(10));
    /// }
    /// ```
    fn initialize_defaults(&mut self) {
        // Default implementation does nothing.
        // Override in concrete implementations.
    }

    /// Removes a singleton of type `T` with the given name, returning it if it existed.
    ///
    /// # Type Parameters
    /// - `T`: The type of singleton to remove.
    /// - `N`: The name type.
    fn remove_singleton<T, N>(&mut self, name: N) -> Option<T>
    where
        T: 'static,
        N: Into<Cow<'static, str>>;

    /// Clears all singletons from the registry, effectively resetting the factory.
    fn clear_all_singletons(&mut self);

    /// Checks whether a singleton of the given type and name is already registered.
    fn contains<T, N>(&self, name: N) -> bool
    where
        T: 'static,
        N: Into<Cow<'static, str>>;

    /// Returns an iterator over all registered singleton names regardless of type.
    fn get_all_names(&self) -> impl Iterator<Item = &Cow<'static, str>>;

    /// Returns the total number of registered singletons.
    fn count(&self) -> usize;

    /// Returns `true` if the factory has no singletons registered.
    fn is_empty(&self) -> bool {
        self.count() == 0
    }
}
