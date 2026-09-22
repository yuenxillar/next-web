use std::collections::HashMap;

use crate::env::{ConfigurablePropertyResolver, Environment, MutablePropertySources};

/// Configuration trait to be implemented by most if not all `Environment` types.
/// Provides facilities for setting active and default profiles and manipulating underlying
/// property sources. Allows clients to set and validate required properties, customize the
/// conversion service and more through the `ConfigurablePropertyResolver` supertrait.
///
/// ## Manipulating property sources
///
/// Property sources may be removed, reordered, or replaced; and additional
/// property sources may be added using the `MutablePropertySources`
/// instance returned from `property_sources()`. The following examples
/// are against the `StandardEnvironment` implementation of
/// `ConfigurableEnvironment`, but are generally applicable to any implementation,
/// though particular default property sources may differ.
///
/// ### Example: adding a new property source with highest search priority
///
/// ```rust
/// use next_web_core::env::{
///     ConfigurableEnvironment, MapPropertySource, PropertyResolver, StandardEnvironment,
///     SYSTEM_ENVIRONMENT_PROPERTY_SOURCE_NAME,
/// };
/// use next_web_core::util::indexmap::IndexMap;
///
/// let mut environment = StandardEnvironment::new();
/// let properties = IndexMap::from([("xyz".to_owned(), "myValue".to_owned())]);
///
/// environment
///     .property_sources()
///     .add_first(Box::new(MapPropertySource::new("MY_MAP".to_owned(), properties)));
///
/// assert_eq!(environment.get_property("xyz"), Some("myValue".to_owned()));
/// ```
///
/// ### Example: removing the property source of the process environment
///
/// ```rust
/// use next_web_core::env::{
///     ConfigurableEnvironment, StandardEnvironment,
///     SYSTEM_ENVIRONMENT_PROPERTY_SOURCE_NAME,
/// };
///
/// let mut environment = StandardEnvironment::new();
/// environment
///     .property_sources()
///     .remove(SYSTEM_ENVIRONMENT_PROPERTY_SOURCE_NAME);
///
/// assert!(!environment
///     .property_sources_ref()
///     .contains(SYSTEM_ENVIRONMENT_PROPERTY_SOURCE_NAME));
/// ```
///
/// ### Example: replacing the property source of the process environment
///
/// ```rust
/// use next_web_core::env::{
///     ConfigurableEnvironment, MapPropertySource, PropertyResolver, StandardEnvironment,
///     SYSTEM_ENVIRONMENT_PROPERTY_SOURCE_NAME,
/// };
/// use next_web_core::util::indexmap::IndexMap;
///
/// let mut environment = StandardEnvironment::new();
/// let mock_environment = IndexMap::from([("xyz".to_owned(), "myValue".to_owned())]);
///
/// environment.property_sources().replace(
///     SYSTEM_ENVIRONMENT_PROPERTY_SOURCE_NAME,
///     Box::new(MapPropertySource::new(
///         SYSTEM_ENVIRONMENT_PROPERTY_SOURCE_NAME.to_owned(),
///         mock_environment,
///     )),
/// );
///
/// assert_eq!(environment.get_property("xyz"), Some("myValue".to_owned()));
/// ```
///
/// When an `Environment` is being used by an application context, it is
/// important that any such `PropertySource` manipulations be performed
/// *before* the context's `refresh()` method is called. This ensures that all
/// property sources are available during the container bootstrap process, including
/// use by property placeholder configurers.
pub trait ConfigurableEnvironment
where
    Self: Environment + ConfigurablePropertyResolver,
    Self: Send + Sync,
{
    /// Specify the set of profiles active for this `Environment`. Profiles are
    /// evaluated during container bootstrap to determine whether bean definitions
    /// should be registered with the container.
    ///
    /// Any existing active profiles will be replaced with the given arguments; call
    /// with an empty slice to clear the current set of active profiles. Use
    /// `add_active_profile` to add a profile while preserving the existing set.
    ///
    /// # Panics
    ///
    /// Panics if any profile is empty or whitespace-only.
    ///
    /// # See also
    ///
    /// - [`add_active_profile`](Self::add_active_profile)
    /// - [`set_default_profiles`](Self::set_default_profiles)
    /// - [`AbstractEnvironment::ACTIVE_PROFILES_PROPERTY_NAME`]
    fn set_active_profiles(&mut self, profiles: &[&str]);

    /// Add a profile to the current set of active profiles.
    ///
    /// # Panics
    ///
    /// Panics if the profile is empty or whitespace-only.
    ///
    /// # See also
    ///
    /// - [`set_active_profiles`](Self::set_active_profiles)
    fn add_active_profile(&mut self, profile: &str);

    /// Specify the set of profiles to be made active by default if no other profiles
    /// are explicitly made active through [`set_active_profiles`](Self::set_active_profiles).
    ///
    /// # Panics
    ///
    /// Panics if any profile is empty or whitespace-only.
    ///
    /// # See also
    ///
    /// - [`AbstractEnvironment::DEFAULT_PROFILES_PROPERTY_NAME`]
    fn set_default_profiles(&mut self, profiles: &[&str]);

    /// Return the `PropertySources` for this `Environment` in mutable form,
    /// allowing for manipulation of the set of `PropertySource` objects that should
    /// be searched when resolving properties against this `Environment` object.
    /// The various `MutablePropertySources` methods such as
    /// `add_first`, `add_last`, `add_before` and `add_after` allow for fine-grained
    /// control over property source ordering. This is useful, for example, in ensuring
    /// that certain user-defined property sources have search precedence over default
    /// property sources such as the set of system properties or the set of system
    /// environment variables.
    fn property_sources(&mut self) -> &mut MutablePropertySources;

    /// Return the `PropertySources` for this `Environment` in immutable form.
    ///
    /// Use [`property_sources`](Self::property_sources) when the property
    /// sources have to be manipulated, for example to remove, reorder or
    /// replace them. This method is meant for the far more common case of
    /// reading the property sources, which does not require mutable access to
    /// the environment.
    fn property_sources_ref(&self) -> &MutablePropertySources;

    /// Return the system properties as a map.
    ///
    /// Note that most `Environment` implementations will include this system
    /// properties map as a default `PropertySource` to be searched. Therefore, it is
    /// recommended that this method not be used directly unless bypassing other
    /// property sources is expressly intended.
    fn system_properties(&self) -> HashMap<String, String>;

    /// Return the system environment variables as a map.
    ///
    /// Note that most `Environment` implementations will include this system
    /// environment map as a default `PropertySource` to be searched. Therefore, it
    /// is recommended that this method not be used directly unless bypassing other
    /// property sources is expressly intended.
    fn system_environment(&self) -> HashMap<String, String>;

    /// Append the given parent environment's active profiles, default profiles, and
    /// property sources to this (child) environment's respective collections of each.
    ///
    /// For any identically-named `PropertySource` instance existing in both
    /// parent and child, the child instance is to be preserved and the parent instance
    /// discarded. This has the effect of allowing overriding of property sources by the
    /// child as well as avoiding redundant searches through common property source types
    /// — for example, system environment and system properties.
    ///
    /// Active and default profile names are also filtered for duplicates, to avoid
    /// confusion and redundant storage.
    ///
    /// The parent environment remains unmodified in any case. Note that any changes to
    /// the parent environment occurring after the call to `merge` will not be
    /// reflected in the child. Therefore, care should be taken to configure parent
    /// property sources and profile information prior to calling `merge`.
    fn merge(&mut self, parent: &dyn ConfigurableEnvironment);
}
