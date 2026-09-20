use std::borrow::Cow;

use crate::factory::config::SingletonRegistry;
use crate::factory::support::DefaultSingletonRegistry;
use crate::factory::support::default_singleton_registry::{DynSingle, Key, Single};
use crate::factory::{ListableSingletonFactory, SingletonFactory};

/// Default [`ListableSingletonFactory`] implementation backed by a
/// [`DefaultSingletonRegistry`].
///
/// The factory owns its registry and provides lazy creation, lookup,
/// enumeration, and removal of named singletons.
pub struct DefaultListableSingletonFactory {
    registry: DefaultSingletonRegistry,
}

impl DefaultListableSingletonFactory {
    /// Creates a new, empty factory.
    pub fn new() -> Self {
        Self {
            registry: DefaultSingletonRegistry::default(),
        }
    }

    pub fn set_allow_override(&mut self, allow_overrides: bool) {
        todo!()
    }
}

impl Default for DefaultListableSingletonFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl ListableSingletonFactory for DefaultListableSingletonFactory {
    fn get_singleton_names_for_type<T>(&self) -> Vec<&str>
    where
        T: 'static,
    {
        self.registry
            .get_singleton_names::<T>()
            .map(|name| name.as_ref())
            .collect()
    }

    fn get_singletons_of_type<T>(&self) -> Vec<&T>
    where
        T: 'static,
    {
        self.registry
            .inner()
            .values()
            .filter_map(DynSingle::as_single::<T>)
            .map(Single::get_ref)
            .collect()
    }

    fn get_singletons_mut_of_type<T>(&mut self) -> Vec<&mut T>
    where
        T: 'static,
    {
        self.registry
            .inner_mut()
            .values_mut()
            .filter_map(DynSingle::as_single_mut::<T>)
            .map(Single::get_mut)
            .collect()
    }
}

impl SingletonFactory for DefaultListableSingletonFactory {
    type Registry = DefaultSingletonRegistry;

    fn registry(&self) -> &Self::Registry {
        &self.registry
    }

    fn registry_mut(&mut self) -> &mut Self::Registry {
        &mut self.registry
    }

    fn remove_singleton<T, N>(&mut self, name: N) -> Option<T>
    where
        T: 'static,
        N: Into<Cow<'static, str>>,
    {
        let key = Key::new::<T>(name.into());
        self.registry
            .remove(&key)
            .and_then(DynSingle::into_single::<T>)
            .map(Single::into_inner)
    }

    fn clear_all_singletons(&mut self) {
        self.registry.clear();
    }

    fn contains<T, N>(&self, name: N) -> bool
    where
        T: 'static,
        N: Into<Cow<'static, str>>,
    {
        self.registry.contains_singleton::<T, N>(name)
    }

    fn get_all_names(&self) -> impl Iterator<Item = &Cow<'static, str>> {
        self.registry.get_all_singleton_names()
    }

    fn count(&self) -> usize {
        self.registry.get_singleton_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq, Clone, Default)]
    struct Config {
        value: i32,
    }

    #[derive(Debug, PartialEq, Eq, Clone, Default)]
    struct Service {
        name: String,
    }

    #[test]
    fn new_factory_is_empty() {
        let factory = DefaultListableSingletonFactory::new();
        assert!(factory.is_empty());
        assert_eq!(factory.count(), 0);
    }

    #[test]
    fn default_trait_creates_empty_factory() {
        let factory = DefaultListableSingletonFactory::default();
        assert!(factory.is_empty());
    }

    #[test]
    fn get_or_create_registers_and_returns() {
        let mut factory = DefaultListableSingletonFactory::new();

        let config = factory.get_or_create("config", || Config { value: 1 });
        assert_eq!(config.value, 1);
        assert_eq!(factory.count(), 1);

        // The existing instance is returned and the factory is not invoked again.
        let config = factory.get_or_create("config", || Config { value: 2 });
        assert_eq!(config.value, 1);
        assert_eq!(factory.count(), 1);
    }

    #[test]
    fn get_or_create_mut_allows_mutation() {
        let mut factory = DefaultListableSingletonFactory::new();
        factory.get_or_create_mut("config", Config::default).value = 10;

        let config: &Config = factory.get_or_create("config", Config::default);
        assert_eq!(config.value, 10);
    }

    #[test]
    fn get_or_default_uses_default() {
        let mut factory = DefaultListableSingletonFactory::new();
        let service: &Service = factory.get_or_default("service");
        assert_eq!(service.name, "");
    }

    #[test]
    fn contains_checks_type_and_name() {
        let mut factory = DefaultListableSingletonFactory::new();
        factory.get_or_create("config", Config::default);

        assert!(factory.contains::<Config, _>("config"));
        assert!(!factory.contains::<Service, _>("config"));
        assert!(!factory.contains::<Config, _>("other"));
    }

    #[test]
    fn get_all_names_lists_every_registered_name() {
        let mut factory = DefaultListableSingletonFactory::new();
        factory.get_or_create("config", Config::default);
        factory.get_or_create("service", Service::default);

        let names: Vec<&str> = factory.get_all_names().map(|name| name.as_ref()).collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"config"));
        assert!(names.contains(&"service"));
    }

    #[test]
    fn get_singletons_of_type_filters_by_type() {
        let mut factory = DefaultListableSingletonFactory::new();
        factory.get_or_create("c1", || Config { value: 1 });
        factory.get_or_create("c2", || Config { value: 2 });
        factory.get_or_create("s1", Service::default);

        let mut configs: Vec<i32> = factory
            .get_singletons_of_type::<Config>()
            .into_iter()
            .map(|config| config.value)
            .collect();
        configs.sort();
        assert_eq!(configs, vec![1, 2]);

        let services = factory.get_singletons_of_type::<Service>();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, "");
    }

    #[test]
    fn get_singleton_names_for_type_filters_by_type() {
        let mut factory = DefaultListableSingletonFactory::new();
        factory.get_or_create("c1", Config::default);
        factory.get_or_create("c2", Config::default);
        factory.get_or_create("s1", Service::default);

        let mut config_names = factory.get_singleton_names_for_type::<Config>();
        config_names.sort();
        assert_eq!(config_names, vec!["c1", "c2"]);

        let service_names = factory.get_singleton_names_for_type::<Service>();
        assert_eq!(service_names, vec!["s1"]);
    }

    #[test]
    fn remove_singleton_returns_owned_value() {
        let mut factory = DefaultListableSingletonFactory::new();
        factory.get_or_create("config", || Config { value: 7 });

        let removed: Option<Config> = factory.remove_singleton("config");
        assert_eq!(removed, Some(Config { value: 7 }));
        assert!(!factory.contains::<Config, _>("config"));
        assert_eq!(factory.count(), 0);

        let missing: Option<Config> = factory.remove_singleton("config");
        assert!(missing.is_none());
    }

    #[test]
    fn clear_all_singletons_empties_factory() {
        let mut factory = DefaultListableSingletonFactory::new();
        factory.get_or_create("config", Config::default);
        factory.get_or_create("service", Service::default);
        assert_eq!(factory.count(), 2);

        factory.clear_all_singletons();
        assert!(factory.is_empty());
        assert!(factory.get_singletons_of_type::<Config>().is_empty());
        assert!(factory.get_all_names().next().is_none());
    }

    #[test]
    fn same_name_different_types_are_independent() {
        let mut factory = DefaultListableSingletonFactory::new();
        factory.get_or_create("item", || Config { value: 1 });
        factory.get_or_create("item", || Service {
            name: "svc".to_owned(),
        });

        assert_eq!(factory.count(), 2);
        assert_eq!(factory.get_or_create("item", Config::default).value, 1);
        assert_eq!(factory.get_or_create("item", Service::default).name, "svc");
    }

    #[test]
    fn remove_singleton_keeps_other_types_with_same_name() {
        let mut factory = DefaultListableSingletonFactory::new();
        factory.get_or_create("item", || Config { value: 1 });
        factory.get_or_create("item", Service::default);

        let removed: Option<Config> = factory.remove_singleton("item");
        assert_eq!(removed, Some(Config { value: 1 }));
        assert!(factory.contains::<Service, _>("item"));
        assert!(!factory.contains::<Config, _>("item"));
    }
}
