use std::borrow::Cow;

/// A registry for managing named singleton instances.
///
/// Allows registration, retrieval, and inspection of singletons identified
/// by names. Each name-type pair corresponds to a unique singleton instance.
pub trait SingletonRegistry {
    /// Registers a singleton instance with the given name.
    ///
    /// If a singleton with the same name and type already exists, it will be replaced.
    ///
    /// # Type Parameters
    /// - `T`: The type of the singleton. Must be `'static` to ensure it lives long enough.
    /// - `N`: The name type, which can be converted into a `Cow<'static, str>`.
    ///
    /// # Examples
    /// ```ignore
    /// registry.register_singleton("config", Config::default());
    /// registry.register_singleton(String::from("logger"), Logger::new());
    /// ```
    fn register_singleton<T, N>(&mut self, name: N, singleton: T)
    where
        T: 'static,
        N: Into<Cow<'static, str>>;

    /// Returns a shared reference to the singleton identified by the given name and type.
    ///
    /// Returns `None` if no singleton of type `T` is registered with the given name.
    ///
    /// # Type Parameters
    /// - `T`: The type of the singleton to retrieve.
    /// - `N`: The name type, which can be converted into a `Cow<'static, str>`.
    fn get_singleton<T, N>(&mut self, name: N) -> Option<&T>
    where
        T: 'static,
        N: Into<Cow<'static, str>>;

    /// Returns a mutable reference to the singleton identified by the given name and type.
    ///
    /// Returns `None` if no singleton of type `T` is registered with the given name.
    ///
    /// # Type Parameters
    /// - `T`: The type of the singleton to retrieve mutably.
    /// - `N`: The name type, which can be converted into a `Cow<'static, str>`.
    fn get_singleton_mut<T, N>(&mut self, name: N) -> Option<&mut T>
    where
        T: 'static,
        N: Into<Cow<'static, str>>;

    /// Checks whether a singleton of the given type and name is registered.
    ///
    /// Returns `true` if a singleton of type `T` with the given name exists.
    ///
    /// # Type Parameters
    /// - `T`: The type of the singleton to check.
    /// - `N`: The name type, which can be converted into a `Cow<'static, str>`.
    fn contains_singleton<T, N>(&self, name: N) -> bool
    where
        T: 'static,
        N: Into<Cow<'static, str>>;

    /// Returns a mutable reference to the singleton with the given name, inserting the
    /// provided default value if it does not already exist.
    ///
    /// This method is a combined "get or insert" operation. If a singleton of type `T`
    /// with the specified name is already registered, a mutable reference to it is returned.
    /// Otherwise, the given `default` value is registered under the name and a mutable
    /// reference to the newly inserted singleton is returned.
    ///
    /// # Type Parameters
    /// - `T`: The type of the singleton. Must be `'static`.
    /// - `N`: The name type, which can be converted into a `Cow<'static, str>`.
    ///
    /// # Arguments
    /// - `name`: The name to identify the singleton. Converted into a `Cow<'static, str>`.
    /// - `default`: The value to insert if no existing singleton is found.
    ///
    /// # Returns
    /// A mutable reference (`&mut T`) to the singleton, whether newly inserted or
    /// previously registered.
    fn get_singleton_or_insert<T, N>(&mut self, name: N, default: T) -> &mut T
    where
        T: 'static,
        N: Into<Cow<'static, str>>;

    /// Returns an iterator over the names of all registered singletons of type `T`.
    ///
    /// The iterator yields references to the names (`&Cow<'static, str>`), avoiding
    /// unnecessary allocations. Callers can `.cloned()` or `.collect()` if owned
    /// values are needed.
    ///
    /// # Type Parameters
    /// - `T`: The type of singletons whose names should be returned.
    fn get_singleton_names<T>(&self) -> impl Iterator<Item = &Cow<'static, str>>
    where
        T: 'static;

    /// Returns an iterator over the names of all registered singletons, regardless of type.
    ///
    /// This provides a view of every singleton name currently stored in the registry.
    /// The iterator yields references to the names (`&Cow<'static, str>`), avoiding
    /// unnecessary allocations.
    ///
    /// # Examples
    /// ```ignore
    /// for name in registry.get_all_singleton_names() {
    ///     println!("Singleton: {}", name);
    /// }
    /// ```
    fn get_all_singleton_names(&self) -> impl Iterator<Item = &Cow<'static, str>>;

    /// Returns the total number of registered singletons across all types.
    fn get_singleton_count(&self) -> usize;
}
