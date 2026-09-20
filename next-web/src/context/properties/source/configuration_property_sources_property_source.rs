//! A property source that adapts configuration property sources.

use std::fmt;

use next_web_core::env::{PropertySource, PropertySourceValue};

use crate::{
    context::properties::source::{ConfigurationPropertyName, NextConfigurationPropertySources},
    env::PropertySourceInfo,
};

/// A [`PropertySource`] that adapts
/// [`ConfigurationPropertySource`](crate::context::properties::source::ConfigurationPropertySource)
/// instances, so that they take part in classic property resolution.
///
/// This is the Rust equivalent of Next Boot's
/// `ConfigurationPropertySourcesPropertySource`. Like the Java version it is
/// immutable and backed by an empty map; its values are resolved through the
/// adapted configuration property sources.
pub struct ConfigurationPropertySourcesPropertySource {
    name: String,
    /// The empty map backing this source, mirroring the Java implementation.
    properties: PropertySourceValue,
    sources: NextConfigurationPropertySources,
}

impl ConfigurationPropertySourcesPropertySource {
    /// Creates a new source with the given name, adapting the given sources.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of this property source.
    /// * `sources` - The adapted configuration property sources.
    pub fn new(name: impl Into<String>, sources: NextConfigurationPropertySources) -> Self {
        Self {
            name: name.into(),
            properties: PropertySourceValue::new(),
            sources,
        }
    }

    /// Returns the adapted configuration property sources.
    pub fn get_source(&self) -> &NextConfigurationPropertySources {
        &self.sources
    }
}

impl PropertySource<PropertySourceValue> for ConfigurationPropertySourcesPropertySource {
    fn name(&self) -> &str {
        &self.name
    }

    /// Returns the value of the configuration property with the given name.
    ///
    /// Names that are not valid configuration property names resolve to `None`,
    /// where the Java implementation raises an
    /// `InvalidConfigurationPropertyNameException`.
    fn property(&self, name: &str) -> Option<String> {
        let name = ConfigurationPropertyName::of(name).ok()?;

        self.sources
            .iter()
            .find_map(|source| source.get_configuration_property(&name))
            .map(|property| property.get_value().to_string())
    }

    fn contains_property(&self, name: &str) -> bool {
        self.property(name).is_some()
    }

    fn source(&self) -> &PropertySourceValue {
        &self.properties
    }
}

impl PropertySourceInfo for ConfigurationPropertySourcesPropertySource {
    /// Always returns `true`.
    fn is_immutable(&self) -> bool {
        true
    }
}

impl fmt::Debug for ConfigurationPropertySourcesPropertySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ConfigurationPropertySourcesPropertySource {{name='{}'}}",
            self.name
        )
    }
}
