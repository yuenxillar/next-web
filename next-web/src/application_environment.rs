//! The environment of an application.

use std::ops::{Deref, DerefMut};

use next_web_core::env::{
    MutablePropertySources, PropertyLookup, PropertySourcesLookup, StandardEnvironment,
};
use next_web_core::impl_environment_delegate;

use crate::context::properties::source::{
    ConfigurationPropertyName, NextConfigurationPropertySources,
};

/// The [`Environment`](next_web_core::env::Environment) that is used by a
/// [`NextWebApplication`](crate::NextWebApplication).
///
/// The environment is a [`StandardEnvironment`] with two differences:
///
/// - its properties are looked up through the configuration property sources of
///   the environment, so that the property names are handled in the same way as
///   they are by configuration properties;
/// - its profiles are never taken from the properties that declare them. The
///   profiles are resolved from the config data by the
///   [`ConfigDataEnvironmentPostProcessor`](crate::context::config::ConfigDataEnvironmentPostProcessor),
///   which applies them to the environment.
pub struct ApplicationEnvironment {
    base: StandardEnvironment,
}

impl ApplicationEnvironment {
    /// Creates a new application environment.
    pub fn new() -> Self {
        Self {
            base: StandardEnvironment::with_lookup(Box::new(ConfigurationPropertyLookup), false),
        }
    }
}

impl Default for ApplicationEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for ApplicationEnvironment {
    type Target = StandardEnvironment;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for ApplicationEnvironment {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl_environment_delegate!(ApplicationEnvironment, base);

/// The [`PropertyLookup`] that is used by an application environment.
///
/// The property sources of the environment are adapted to configuration
/// property sources, which are searched in order. Sources that hold no
/// properties, and the adapter of the configuration properties itself, are
/// skipped by the adaptation.
///
/// A key that is not a configuration property name, such as the name of an
/// environment variable, is looked up as it is.
#[derive(Debug, Clone, Copy, Default)]
struct ConfigurationPropertyLookup;

impl PropertyLookup for ConfigurationPropertyLookup {
    fn lookup(&self, property_sources: &MutablePropertySources, key: &str) -> Option<String> {
        let Ok(name) = ConfigurationPropertyName::of(key) else {
            return PropertySourcesLookup.lookup(property_sources, key);
        };

        NextConfigurationPropertySources::new(property_sources)
            .iter()
            .find_map(|source| source.get_configuration_property(&name))
            .map(|property| property.get_value().to_string())
    }
}

#[cfg(test)]
mod tests {
    use next_web_core::env::{
        ConfigurableEnvironment, Environment, PropertyResolver, RESERVED_DEFAULT_PROFILE_NAME,
        SYSTEM_ENVIRONMENT_PROPERTY_SOURCE_NAME,
    };
    use next_web_core::util::indexmap::IndexMap;

    use crate::env::MapPropertySource;

    use super::*;

    /// Adds a map backed property source with the given properties.
    fn add_source(environment: &mut ApplicationEnvironment, properties: &[(&str, &str)]) {
        let properties: IndexMap<String, String> = properties
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        environment
            .property_sources()
            .add_first(Box::new(MapPropertySource::new(
                "test".to_owned(),
                properties,
            )));
    }

    #[test]
    fn adds_the_property_sources_of_a_standard_environment() {
        let environment = ApplicationEnvironment::default();

        let names: Vec<&str> = environment
            .property_sources_ref()
            .iter()
            .map(|property_source| property_source.name())
            .collect();
        assert_eq!(names, vec![SYSTEM_ENVIRONMENT_PROPERTY_SOURCE_NAME]);
    }

    #[test]
    fn resolves_properties_from_the_configuration_property_sources() {
        let mut environment = ApplicationEnvironment::default();
        add_source(&mut environment, &[("next.application.name", "demo")]);

        assert!(environment.contains_property("next.application.name"));
        assert_eq!(
            environment.get_property("next.application.name"),
            Some("demo".to_owned())
        );
        assert_eq!(environment.get_property("next.application.version"), None);
    }

    #[test]
    fn resolves_the_name_of_an_environment_variable_as_it_is() {
        let mut environment = ApplicationEnvironment::default();
        add_source(&mut environment, &[("PATH", "a-path")]);

        assert_eq!(environment.get_property("PATH"), Some("a-path".to_owned()));
    }

    #[test]
    fn does_not_take_the_profiles_from_the_profile_properties() {
        let mut environment = ApplicationEnvironment::default();
        add_source(
            &mut environment,
            &[
                ("next.profiles.active", "dev"),
                ("next.profiles.default", "cloud"),
            ],
        );

        assert_eq!(environment.active_profiles(), &[] as &[String]);
        assert_eq!(
            environment.default_profiles(),
            &[RESERVED_DEFAULT_PROFILE_NAME.to_owned()]
        );

        environment.set_active_profiles(&["dev"]);

        assert_eq!(environment.active_profiles(), &["dev".to_owned()]);
    }

    #[test]
    fn resolves_the_placeholders_of_its_properties() {
        let mut environment = ApplicationEnvironment::default();
        add_source(
            &mut environment,
            &[
                ("next.application.name", "demo"),
                (
                    "next.application.description",
                    "the ${next.application.name} application",
                ),
            ],
        );

        assert_eq!(
            environment.get_property("next.application.description"),
            Some("the demo application".to_owned())
        );
    }
}
