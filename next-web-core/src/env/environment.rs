use crate::env::{EnvError, Profiles, PropertyResolver, profiles_of};

/// Trait representing the environment in which the current application
/// is running.
///
/// Models two key aspects of the application environment: *profiles* and
/// *properties*. Methods related to property access are exposed via the
/// [`PropertyResolver`] supertrait.
///
/// # Profiles
///
/// A *profile* is a named, logical group of bean definitions to be registered
/// with the container only if the given profile is *active*. The role of the
/// `Environment` with relation to profiles is in determining which profiles
/// (if any) are currently [`active`](Self::active_profiles), and which
/// profiles (if any) should be [`active by default`](Self::default_profiles).
///
/// # Properties
///
/// *Properties* play an important role in almost all applications, and may
/// originate from a variety of sources: properties files
/// properties, system environment variables, servlet context parameters,
/// ad-hoc `Properties` objects, maps, and so on. The role of the `Environment`
/// with relation to properties is to provide the user with a convenient
/// service interface for configuring property sources and resolving properties
/// from them.
///
/// # Usage
///
/// In most cases, application-level components should not need to interact
/// with the `Environment` directly but instead may request to have `${...}`
/// property values replaced by a property placeholder configurer, which
/// itself is environment-aware.
///
/// Configuration of the `Environment` must be done through the
/// [`ConfigurableEnvironment`] trait, returned from all application context
/// `environment()` methods. See [`ConfigurableEnvironment`] for usage examples
/// demonstrating manipulation of property sources prior to application context
/// refresh.
///
/// [`PropertyResolver`]: crate::env::PropertyResolver
/// [`ConfigurableEnvironment`]: crate::env::ConfigurableEnvironment
pub trait Environment: PropertyResolver {
    /// Return the set of profiles explicitly made active for this environment.
    ///
    /// Profiles are used for creating logical groupings of component
    /// definitions to be registered conditionally, for example based on
    /// deployment environment. Profiles can be activated by setting
    /// `"spring.profiles.active"` as a system property or by calling
    /// [`ConfigurableEnvironment::set_active_profiles`].
    ///
    /// If no profiles have explicitly been specified as active, then any
    /// [`default profiles`](Self::default_profiles) will automatically be
    /// activated.
    ///
    /// [`ConfigurableEnvironment::set_active_profiles`]:
    ///     crate::env::ConfigurableEnvironment::set_active_profiles
    fn active_profiles(&self) -> &[String];

    /// Return the set of profiles to be active by default when no active
    /// profiles have been set explicitly.
    fn default_profiles(&self) -> &[String];

    /// Determine whether one of the given profile expressions matches the
    /// [`active profiles`](Self::active_profiles) — or in the case of no
    /// explicit active profiles, whether one of the given profile expressions
    /// matches the [`default profiles`](Self::default_profiles).
    ///
    /// Profile expressions allow for complex, boolean profile logic to be
    /// expressed — for example `"p1 & p2"`, `"(p1 & p2) | p3"`, etc. See
    /// [`Profiles::of`] for details on the supported expression syntax.
    ///
    /// This method is a convenient shortcut for
    /// `env.accepts_profiles(&Profiles::of(profile_expressions)?)`.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if any expression is malformed, or
    /// [`EnvError`] if no active and no default profiles are configured.
    ///
    /// [`Profiles::of`]: crate::env::Profiles::of
    fn matches_profiles(&self, profile_expressions: &[&str]) -> Result<bool, EnvError> {
        Ok(self.accepts_profiles(profiles_of(profile_expressions)?.as_ref()))
    }

    /// Determine whether the given [`Profiles`] predicate matches the
    /// [`active profiles`](Self::active_profiles) — or in the case of no
    /// explicit active profiles, whether the given `Profiles` predicate
    /// matches the [`default profiles`](Self::default_profiles).
    ///
    /// If you wish to provide profile expressions directly as strings, use
    /// [`matches_profiles`](Self::matches_profiles) instead.
    fn accepts_profiles(&self, profiles: &dyn Profiles) -> bool;
}
