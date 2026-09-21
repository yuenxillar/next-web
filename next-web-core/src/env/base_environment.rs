//! Base implementation of an environment.

use std::collections::HashMap;
use std::sync::OnceLock;

use tracing::debug;

use crate::env::{
    ConfigurableEnvironment, ConfigurablePropertyResolver, Environment, MapPropertySource,
    MutablePropertySources, PlaceholdersResolver, Profiles, PropertyLookup, PropertyResolver,
    PropertySourceValue, PropertySourcesLookup,
};
use crate::error::IllegalError;
use crate::util::indexmap::IndexMap;

/// Property that instructs the framework to ignore the environment variables of
/// the process.
///
/// When the property is `true`, the environment variables are not read, so that
/// a property that cannot be resolved otherwise does not fall back to them.
pub const IGNORE_GETENV_PROPERTY_NAME: &str = "next.getenv.ignore";

/// Name of the property that specifies the active profiles.
///
/// The value of the property is a comma delimited list of profile names.
pub const ACTIVE_PROFILES_PROPERTY_NAME: &str = "next.profiles.active";

/// Name of the property that specifies the profiles that are active by default.
///
/// The value of the property is a comma delimited list of profile names.
pub const DEFAULT_PROFILES_PROPERTY_NAME: &str = "next.profiles.default";

/// Name of the profile that is active when no other profile is active.
pub const RESERVED_DEFAULT_PROFILE_NAME: &str = "default";

/// The profiles that were resolved from the properties of an environment.
///
/// The profiles are resolved once, the first time they are requested, so that an
/// environment that has no profiles of its own does not look the profile
/// properties up on every access.
#[derive(Debug, Default, PartialEq, Eq)]
struct ResolvedProfiles {
    active: Vec<String>,
    default: Vec<String>,
}

/// Base implementation of an [`Environment`].
///
/// The base implementation adds no property sources. It supports a reserved
/// default profile name and allows the active and the default profiles to be
/// declared by the [`ACTIVE_PROFILES_PROPERTY_NAME`] and
/// [`DEFAULT_PROFILES_PROPERTY_NAME`] properties. Environments that add property
/// sources compose it: see
/// [`StandardEnvironment`](crate::env::StandardEnvironment), and
/// [`impl_environment_delegate`](crate::impl_environment_delegate) for the
/// traits that such an environment has to implement.
///
/// The properties of the environment are looked up by the [`PropertyLookup`]
/// that is given at construction, while the placeholders of the values that the
/// lookup returns are resolved by the environment itself.
pub struct BaseEnvironment {
    active_profiles: Vec<String>,
    default_profiles: Vec<String>,
    resolved_profiles: OnceLock<ResolvedProfiles>,
    property_sources: MutablePropertySources,
    lookup: Box<dyn PropertyLookup>,
    placeholders_resolver: PlaceholdersResolver,
    read_profile_properties: bool,
}

impl BaseEnvironment {
    /// Creates an environment that adds no property sources.
    ///
    /// The active and default profiles are taken from the properties that
    /// declare them, unless they are set explicitly.
    pub fn new() -> Self {
        Self::with_lookup(Box::new(PropertySourcesLookup), true)
    }

    /// Creates an environment that adds no property sources and that looks its
    /// properties up with the given strategy.
    ///
    /// # Arguments
    ///
    /// * `lookup` - The strategy used to look the properties up.
    /// * `read_profile_properties` - Whether the active and default profiles may
    ///   be taken from the properties that declare them. An environment that
    ///   resolves the profiles itself passes `false`, so that the profiles are
    ///   only the ones that are set on it.
    pub fn with_lookup(lookup: Box<dyn PropertyLookup>, read_profile_properties: bool) -> Self {
        Self {
            active_profiles: Vec::new(),
            default_profiles: vec![RESERVED_DEFAULT_PROFILE_NAME.to_owned()],
            resolved_profiles: OnceLock::new(),
            property_sources: MutablePropertySources::new(),
            lookup,
            placeholders_resolver: PlaceholdersResolver::new(),
            read_profile_properties,
        }
    }

    /// Returns the active profiles of this environment.
    ///
    /// The profiles that the profile property declares are used when no profile
    /// was set explicitly.
    fn resolve_active_profiles(&self) -> &[String] {
        if !self.active_profiles.is_empty() {
            return &self.active_profiles;
        }

        let resolved = &self.resolved_profiles().active;
        if resolved.is_empty() {
            return &self.active_profiles;
        }
        resolved
    }

    /// Returns the profiles that are active by default in this environment.
    ///
    /// The profiles that the profile property declares are used while the
    /// default profiles are still the reserved one.
    fn resolve_default_profiles(&self) -> &[String] {
        if !self.has_only_reserved_default_profiles() {
            return &self.default_profiles;
        }

        let resolved = &self.resolved_profiles().default;
        if resolved.is_empty() {
            return &self.default_profiles;
        }
        resolved
    }

    /// Returns whether the default profiles are still the reserved one.
    fn has_only_reserved_default_profiles(&self) -> bool {
        self.default_profiles.len() == 1
            && self.default_profiles[0] == RESERVED_DEFAULT_PROFILE_NAME
    }

    /// Returns the profiles that are declared by the properties of this
    /// environment, resolving them on the first call.
    fn resolved_profiles(&self) -> &ResolvedProfiles {
        self.resolved_profiles.get_or_init(|| {
            if !self.read_profile_properties {
                return ResolvedProfiles::default();
            }

            ResolvedProfiles {
                active: self.profile_property(ACTIVE_PROFILES_PROPERTY_NAME),
                default: self.profile_property(DEFAULT_PROFILES_PROPERTY_NAME),
            }
        })
    }

    /// Returns the profiles declared by the given profile property.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the profile property.
    fn profile_property(&self, name: &str) -> Vec<String> {
        self.get_property(name)
            .map(|value| profiles_from_value(&value))
            .unwrap_or_default()
    }

    /// Returns whether the given profile is active, or, when no profile is
    /// active at all, whether the given profile is active by default.
    ///
    /// # Arguments
    ///
    /// * `profile` - The name of the profile.
    ///
    /// # Panics
    ///
    /// Panics when the profile has no text or starts with the `!` operator, see
    /// [`validate_profile`].
    fn is_profile_active(&self, profile: &str) -> bool {
        validate_profile(profile);

        let active_profiles = self.resolve_active_profiles();
        contains_profile(active_profiles, profile)
            || (active_profiles.is_empty()
                && contains_profile(self.resolve_default_profiles(), profile))
    }

    /// Returns the values of the environment variables of the process.
    ///
    /// An empty map is returned when the [`IGNORE_GETENV_PROPERTY_NAME`]
    /// property is `true`.
    fn read_system_environment(&self) -> HashMap<String, String> {
        if self
            .get_property(IGNORE_GETENV_PROPERTY_NAME)
            .map(|value| parse_bool(&value).unwrap_or(false))
            .unwrap_or(false)
        {
            debug!("Ignoring the environment variables of the process");
            return HashMap::new();
        }

        std::env::vars().collect()
    }

    /// Returns the closure that is used to resolve the key of a placeholder.
    fn value_lookup(&self) -> impl Fn(&str) -> Option<String> + '_ {
        move |key| self.lookup.lookup(&self.property_sources, key)
    }
}

impl Default for BaseEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyResolver for BaseEnvironment {
    fn contains_property(&self, key: &str) -> bool {
        self.lookup.lookup(&self.property_sources, key).is_some()
    }

    /// Returns the value of the given key, with its placeholders resolved.
    ///
    /// A placeholder of the value that cannot be resolved is kept as it is
    /// written. Use
    /// [`resolve_required_placeholders`](Self::resolve_required_placeholders)
    /// to report such a placeholder as an error.
    fn get_property(&self, key: &str) -> Option<String> {
        let value = self.lookup.lookup(&self.property_sources, key)?;
        let (value, _) = self
            .placeholders_resolver
            .resolve(&value, &self.value_lookup());
        Some(value)
    }

    fn get_property_or_default(&self, key: &str, default_value: &str) -> String {
        self.get_property(key)
            .unwrap_or_else(|| default_value.to_owned())
    }

    /// # Errors
    ///
    /// Returns [`IllegalError`] when the key cannot be resolved.
    fn get_required_property(&self, key: &str) -> Result<String, IllegalError> {
        self.get_property(key)
            .ok_or_else(|| IllegalError::IllegalStateError(format!("Property '{key}' is required")))
    }

    fn resolve_placeholders(&self, text: &str) -> String {
        self.placeholders_resolver
            .resolve(text, &self.value_lookup())
            .0
    }

    /// # Errors
    ///
    /// Returns [`IllegalError`] when a placeholder cannot be resolved.
    fn resolve_required_placeholders(&self, text: &str) -> Result<String, IllegalError> {
        let (value, resolved) = self
            .placeholders_resolver
            .resolve(text, &self.value_lookup());

        if resolved {
            return Ok(value);
        }
        Err(IllegalError::InvalidParameterError(format!(
            "Could not resolve placeholder in \"{text}\""
        )))
    }
}

impl ConfigurablePropertyResolver for BaseEnvironment {}

impl Environment for BaseEnvironment {
    fn active_profiles(&self) -> &[String] {
        self.resolve_active_profiles()
    }

    fn default_profiles(&self) -> &[String] {
        self.resolve_default_profiles()
    }

    fn accepts_profiles(&self, profiles: &dyn Profiles) -> bool {
        profiles.matches(&|profile: &str| self.is_profile_active(profile))
    }
}

impl ConfigurableEnvironment for BaseEnvironment {
    /// Sets the active profiles of this environment, replacing the profiles that
    /// were set before.
    ///
    /// # Arguments
    ///
    /// * `profiles` - The names of the profiles that are active.
    ///
    /// # Panics
    ///
    /// Panics when a profile has no text or starts with the `!` operator, see
    /// [`validate_profile`].
    fn set_active_profiles(&mut self, profiles: &[&str]) {
        debug!("Activating profiles {profiles:?}");
        for profile in profiles {
            validate_profile(profile);
        }

        let mut active_profiles = Vec::with_capacity(profiles.len());
        for profile in profiles {
            if !contains_profile(&active_profiles, profile) {
                active_profiles.push((*profile).to_owned());
            }
        }

        self.active_profiles = active_profiles;
        self.resolved_profiles = OnceLock::new();
    }

    /// Adds a profile to the active profiles of this environment.
    ///
    /// The profiles that the profile property declares are taken over before the
    /// given profile is added to them.
    ///
    /// # Arguments
    ///
    /// * `profile` - The name of the profile.
    ///
    /// # Panics
    ///
    /// Panics when the profile has no text or starts with the `!` operator, see
    /// [`validate_profile`].
    fn add_active_profile(&mut self, profile: &str) {
        debug!("Activating profile '{profile}'");
        validate_profile(profile);

        let resolved = self.resolved_profiles().active.clone();
        if self.active_profiles.is_empty() {
            self.active_profiles = resolved;
        }
        if !contains_profile(&self.active_profiles, profile) {
            self.active_profiles.push(profile.to_owned());
        }

        self.resolved_profiles = OnceLock::new();
    }

    /// Sets the profiles that are active when no profile is active, replacing
    /// the reserved default profile as well.
    ///
    /// # Arguments
    ///
    /// * `profiles` - The names of the profiles that are active by default.
    ///
    /// # Panics
    ///
    /// Panics when a profile has no text or starts with the `!` operator, see
    /// [`validate_profile`].
    fn set_default_profiles(&mut self, profiles: &[&str]) {
        for profile in profiles {
            validate_profile(profile);
        }

        let mut default_profiles = Vec::with_capacity(profiles.len());
        for profile in profiles {
            if !contains_profile(&default_profiles, profile) {
                default_profiles.push((*profile).to_owned());
            }
        }

        self.default_profiles = default_profiles;
        self.resolved_profiles = OnceLock::new();
    }

    fn property_sources(&mut self) -> &mut MutablePropertySources {
        &mut self.property_sources
    }

    fn property_sources_ref(&self) -> &MutablePropertySources {
        &self.property_sources
    }

    /// Returns the system properties of the process.
    ///
    /// Rust has no process wide property table, so the map is empty. The method
    /// is kept because environments declare it, and an environment that has a
    /// property table of its own can override it.
    fn system_properties(&self) -> HashMap<String, String> {
        HashMap::new()
    }

    fn system_environment(&self) -> HashMap<String, String> {
        self.read_system_environment()
    }

    /// Merges the property sources and the profiles of the given parent into
    /// this environment.
    ///
    /// The property sources of the parent are copied, in their own order, after
    /// the property sources that this environment already has. A property source
    /// whose name is already used is not copied, so the property sources of this
    /// environment keep their precedence. The parent is not modified.
    ///
    /// # Arguments
    ///
    /// * `parent` - The environment to merge into this one.
    fn merge(&mut self, parent: &dyn ConfigurableEnvironment) {
        for property_source in parent.property_sources_ref().iter() {
            if self.property_sources.contains(property_source.name()) {
                continue;
            }
            self.property_sources
                .add_last(copy_property_source(property_source));
        }

        for profile in parent.active_profiles() {
            if !contains_profile(&self.active_profiles, profile) {
                self.active_profiles.push(profile.to_owned());
            }
        }

        let parent_default_profiles = parent.default_profiles().to_vec();
        if !parent_default_profiles.is_empty() {
            self.default_profiles
                .retain(|profile| profile != RESERVED_DEFAULT_PROFILE_NAME);
            for profile in parent_default_profiles {
                if !contains_profile(&self.default_profiles, &profile) {
                    self.default_profiles.push(profile);
                }
            }
        }
    }
}

/// Returns whether the given profiles contain the given profile.
///
/// # Arguments
///
/// * `profiles` - The profiles to search.
/// * `profile` - The name of the profile.
fn contains_profile(profiles: &[String], profile: &str) -> bool {
    profiles.iter().any(|candidate| candidate == profile)
}

/// Splits a comma delimited profile property into profile names.
///
/// # Arguments
///
/// * `value` - The value of a profile property.
fn profiles_from_value(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

/// Parses a boolean property value, accepting `true` and `false` in any case.
///
/// # Arguments
///
/// * `value` - The value to parse.
fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// Validates the given profile name.
///
/// # Arguments
///
/// * `profile` - The name of the profile to validate.
///
/// # Panics
///
/// Panics when the profile has no text, or when it starts with the `!`
/// operator, which is only allowed in a profile expression.
fn validate_profile(profile: &str) {
    assert!(
        !profile.trim().is_empty(),
        "Invalid profile '{profile}': must contain text"
    );
    assert!(
        !profile.starts_with('!'),
        "Invalid profile '{profile}': must not begin with the ! operator"
    );
}

/// Returns an owned copy of the given property source.
///
/// A property source that does not enumerate its property names is copied as an
/// empty source, so that the copy keeps the position of the original in the
/// search order.
///
/// # Arguments
///
/// * `property_source` - The property source to copy.
fn copy_property_source(
    property_source: &dyn crate::env::PropertySource<PropertySourceValue>,
) -> crate::env::BoxedPropertySource {
    let properties: IndexMap<String, String> = property_source
        .property_names()
        .into_iter()
        .filter_map(|name| property_source.property(&name).map(|value| (name, value)))
        .collect();

    Box::new(MapPropertySource::new(
        property_source.name().to_owned(),
        properties,
    ))
}

/// Implements the environment traits for a type that composes another
/// environment, forwarding every method to it.
///
/// An environment that adds behaviour to another environment composes it and
/// forwards the [`ConfigurableEnvironment`](crate::env::ConfigurableEnvironment),
/// [`Environment`](crate::env::Environment),
/// [`PropertyResolver`](crate::env::PropertyResolver) and
/// [`ConfigurablePropertyResolver`](crate::env::ConfigurablePropertyResolver)
/// methods to it.
///
/// # Arguments
///
/// * `$environment` - The type that composes another environment.
/// * `$base` - The name of the field that holds the composed environment.
///
/// # Examples
///
/// ```
/// use std::ops::{Deref, DerefMut};
///
/// use next_web_core::env::{BaseEnvironment, StandardEnvironment};
/// use next_web_core::impl_environment_delegate;
///
/// /// An environment that composes a standard environment.
/// struct MyEnvironment {
///     base: StandardEnvironment,
/// }
///
/// impl MyEnvironment {
///     fn new() -> Self {
///         Self {
///             base: StandardEnvironment::new(),
///         }
///     }
/// }
///
/// impl Deref for MyEnvironment {
///     type Target = BaseEnvironment;
///
///     fn deref(&self) -> &Self::Target {
///         &self.base
///     }
/// }
///
/// impl DerefMut for MyEnvironment {
///     fn deref_mut(&mut self) -> &mut Self::Target {
///         &mut self.base
///     }
/// }
///
/// impl_environment_delegate!(MyEnvironment, base);
/// ```
#[macro_export]
macro_rules! impl_environment_delegate {
    ($environment:ty, $base:ident) => {
        impl $crate::env::PropertyResolver for $environment {
            fn contains_property(&self, key: &str) -> bool {
                self.$base.contains_property(key)
            }

            fn get_property(&self, key: &str) -> Option<String> {
                self.$base.get_property(key)
            }

            fn get_property_or_default(&self, key: &str, default_value: &str) -> String {
                self.$base.get_property_or_default(key, default_value)
            }

            fn get_required_property(
                &self,
                key: &str,
            ) -> Result<String, $crate::error::IllegalError> {
                self.$base.get_required_property(key)
            }

            fn resolve_placeholders(&self, text: &str) -> String {
                self.$base.resolve_placeholders(text)
            }

            fn resolve_required_placeholders(
                &self,
                text: &str,
            ) -> Result<String, $crate::error::IllegalError> {
                self.$base.resolve_required_placeholders(text)
            }
        }

        impl $crate::env::ConfigurablePropertyResolver for $environment {}

        impl $crate::env::Environment for $environment {
            fn active_profiles(&self) -> &[String] {
                self.$base.active_profiles()
            }

            fn default_profiles(&self) -> &[String] {
                self.$base.default_profiles()
            }

            fn accepts_profiles(&self, profiles: &dyn $crate::env::Profiles) -> bool {
                self.$base.accepts_profiles(profiles)
            }
        }

        impl $crate::env::ConfigurableEnvironment for $environment {
            fn set_active_profiles(&mut self, profiles: &[&str]) {
                self.$base.set_active_profiles(profiles)
            }

            fn add_active_profile(&mut self, profile: &str) {
                self.$base.add_active_profile(profile)
            }

            fn set_default_profiles(&mut self, profiles: &[&str]) {
                self.$base.set_default_profiles(profiles)
            }

            fn property_sources(&mut self) -> &mut $crate::env::MutablePropertySources {
                self.$base.property_sources()
            }

            fn property_sources_ref(&self) -> &$crate::env::MutablePropertySources {
                self.$base.property_sources_ref()
            }

            fn system_properties(&self) -> std::collections::HashMap<String, String> {
                self.$base.system_properties()
            }

            fn system_environment(&self) -> std::collections::HashMap<String, String> {
                self.$base.system_environment()
            }

            fn merge(&mut self, parent: &dyn $crate::env::ConfigurableEnvironment) {
                self.$base.merge(parent)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::env::profiles_of;

    use super::*;

    /// Adds a map backed property source with the given properties.
    fn add_source(environment: &mut BaseEnvironment, name: &str, properties: &[(&str, &str)]) {
        let properties: IndexMap<String, String> = properties
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        environment
            .property_sources()
            .add_last(Box::new(MapPropertySource::new(
                name.to_owned(),
                properties,
            )));
    }

    /// Returns an environment holding the given properties.
    fn environment(properties: &[(&str, &str)]) -> BaseEnvironment {
        let mut environment = BaseEnvironment::new();
        add_source(&mut environment, "test", properties);
        environment
    }

    #[test]
    fn adds_no_property_sources() {
        let environment = BaseEnvironment::new();

        assert!(environment.property_sources_ref().is_empty());
        assert_eq!(environment.active_profiles(), &[] as &[String]);
        assert_eq!(
            environment.default_profiles(),
            &[RESERVED_DEFAULT_PROFILE_NAME.to_owned()]
        );
    }

    #[test]
    fn resolves_the_properties_of_its_property_sources() {
        let environment = environment(&[("next.application.name", "demo")]);

        assert!(environment.contains_property("next.application.name"));
        assert_eq!(
            environment.get_property("next.application.name"),
            Some("demo".to_owned())
        );
        assert_eq!(environment.get_property("next.application.version"), None);
        assert_eq!(
            environment.get_property_or_default("next.application.version", "1.0.0"),
            "1.0.0"
        );
        assert_eq!(
            environment
                .get_required_property("next.application.name")
                .ok(),
            Some("demo".to_owned())
        );
        assert!(environment
            .get_required_property("next.application.version")
            .is_err());
    }

    #[test]
    fn resolves_the_placeholders_of_its_properties_and_of_text() {
        let environment = environment(&[
            ("next.application.name", "demo"),
            (
                "next.application.description",
                "the ${next.application.name} application",
            ),
        ]);

        assert_eq!(
            environment.get_property("next.application.description"),
            Some("the demo application".to_owned())
        );
        assert_eq!(
            environment.resolve_placeholders("the ${next.application.name} application"),
            "the demo application"
        );
        assert_eq!(
            environment.resolve_placeholders("${next.application.missing:fallback}"),
            "fallback"
        );
        assert_eq!(
            environment.resolve_placeholders("${next.application.missing}"),
            "${next.application.missing}"
        );
        assert_eq!(
            environment
                .resolve_required_placeholders("${next.application.name}")
                .ok(),
            Some("demo".to_owned())
        );
        assert!(environment
            .resolve_required_placeholders("${next.application.missing}")
            .is_err());
    }

    #[test]
    fn sets_and_adds_active_profiles() {
        let mut environment = BaseEnvironment::new();

        environment.set_active_profiles(&["dev", "cloud", "dev"]);

        assert_eq!(
            environment.active_profiles(),
            &["dev".to_owned(), "cloud".to_owned()]
        );

        environment.add_active_profile("local");
        environment.add_active_profile("dev");

        assert_eq!(
            environment.active_profiles(),
            &["dev".to_owned(), "cloud".to_owned(), "local".to_owned()]
        );

        environment.set_active_profiles(&[]);

        assert_eq!(environment.active_profiles(), &[] as &[String]);
    }

    #[test]
    #[should_panic(expected = "must contain text")]
    fn rejects_an_empty_profile() {
        BaseEnvironment::new().set_active_profiles(&[""]);
    }

    #[test]
    #[should_panic(expected = "must not begin with the ! operator")]
    fn rejects_a_negated_profile() {
        BaseEnvironment::new().add_active_profile("!dev");
    }

    #[test]
    fn sets_the_default_profiles() {
        let mut environment = BaseEnvironment::new();

        environment.set_default_profiles(&["dev", "cloud", "dev"]);

        assert_eq!(
            environment.default_profiles(),
            &["dev".to_owned(), "cloud".to_owned()]
        );
    }

    #[test]
    fn resolves_the_profiles_from_the_profile_properties() {
        let environment = environment(&[
            (ACTIVE_PROFILES_PROPERTY_NAME, "dev, cloud"),
            (DEFAULT_PROFILES_PROPERTY_NAME, "local"),
        ]);

        assert_eq!(
            environment.active_profiles(),
            &["dev".to_owned(), "cloud".to_owned()]
        );
        assert_eq!(environment.default_profiles(), &["local".to_owned()]);
    }

    #[test]
    fn keeps_an_empty_active_profile_set_when_the_property_is_absent() {
        let environment = environment(&[(DEFAULT_PROFILES_PROPERTY_NAME, "local")]);

        assert_eq!(environment.active_profiles(), &[] as &[String]);
        assert_eq!(environment.default_profiles(), &["local".to_owned()]);
    }

    #[test]
    fn an_explicitly_set_profile_wins_over_the_profile_property() {
        let mut environment = environment(&[(ACTIVE_PROFILES_PROPERTY_NAME, "dev")]);

        environment.set_active_profiles(&["local"]);

        assert_eq!(environment.active_profiles(), &["local".to_owned()]);
    }

    #[test]
    fn reports_whether_a_profile_is_active() {
        let mut environment = environment(&[]);
        environment.set_active_profiles(&["dev", "cloud"]);

        assert!(environment.accepts_profiles(&*profiles_of(&["dev"]).unwrap()));
        assert!(environment.accepts_profiles(&*profiles_of(&["dev & cloud"]).unwrap()));
        assert!(!environment.accepts_profiles(&*profiles_of(&["prod"]).unwrap()));
        assert_eq!(
            environment.matches_profiles(&["dev", "prod"]).ok(),
            Some(true)
        );
        assert_eq!(environment.matches_profiles(&["prod"]).ok(), Some(false));
    }

    #[test]
    fn falls_back_to_the_default_profiles_when_no_profile_is_active() {
        let mut environment = environment(&[]);
        environment.set_default_profiles(&["local"]);

        assert!(environment.accepts_profiles(&*profiles_of(&["local"]).unwrap()));
        assert!(!environment.accepts_profiles(&*profiles_of(&["dev"]).unwrap()));
    }

    #[test]
    fn merges_the_property_sources_and_the_profiles_of_a_parent() {
        let mut child = BaseEnvironment::new();
        add_source(&mut child, "child", &[("next.application.name", "child")]);
        add_source(&mut child, "shared", &[("next.server.port", "9090")]);

        let mut parent = BaseEnvironment::new();
        add_source(
            &mut parent,
            "parent",
            &[("next.application.version", "1.0.0")],
        );
        add_source(&mut parent, "shared", &[("next.server.port", "8080")]);
        parent.set_active_profiles(&["dev"]);
        parent.set_default_profiles(&["cloud"]);

        child.merge(&parent);

        let names: Vec<&str> = child
            .property_sources_ref()
            .iter()
            .map(|property_source| property_source.name())
            .collect();
        // The source named "shared" is the one of the child, the one of the
        // parent is not copied.
        assert_eq!(names, vec!["child", "shared", "parent"]);
        assert_eq!(child.property_sources_ref().len(), 3);
        assert_eq!(
            child.get_property("next.application.name"),
            Some("child".to_owned())
        );
        assert_eq!(
            child.get_property("next.server.port"),
            Some("9090".to_owned())
        );
        assert_eq!(
            child.get_property("next.application.version"),
            Some("1.0.0".to_owned())
        );
        assert_eq!(child.active_profiles(), &["dev".to_owned()]);
        assert_eq!(child.default_profiles(), &["cloud".to_owned()]);

        // The parent is not modified by the merge.
        assert_eq!(parent.active_profiles(), &["dev".to_owned()]);
        assert_eq!(parent.default_profiles(), &["cloud".to_owned()]);
        assert_eq!(parent.property_sources_ref().len(), 2);
    }

    #[test]
    fn ignores_the_environment_variables_of_the_process_when_asked_to() {
        let environment = environment(&[(IGNORE_GETENV_PROPERTY_NAME, "true")]);

        assert!(environment.system_environment().is_empty());
        assert!(environment.system_properties().is_empty());
    }

    #[test]
    fn creates_a_default_environment() {
        let environment = BaseEnvironment::default();

        assert!(environment.property_sources_ref().is_empty());
    }
}
