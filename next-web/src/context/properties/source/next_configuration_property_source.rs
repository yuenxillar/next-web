//! A configuration property source adapted from a classic property source.

use next_web_core::{
    anys::any_value::AnyValue,
    env::{PropertySource, PropertySourceValue},
    util::indexmap::IndexMap,
};

use crate::context::properties::source::{
    ConfigurationProperty, ConfigurationPropertyName, ConfigurationPropertySource,
};

/// A [`ConfigurationPropertySource`] adapted from a [`PropertySource`].
///
/// This is the Rust equivalent of Next Boot's
/// `NextConfigurationPropertySource`. The adapted source holds a snapshot of
/// the properties the underlying source exposes when
/// [`from`](Self::from) is called.
///
/// Note: Next Boot maps the names of system environment sources, looking up
/// `next.datasource.url` as `SPRING_DATASOURCE_URL` for example. Those name
/// mappings are not part of this conversion yet, so names are looked up as-is.
#[derive(Debug, Clone, Default)]
pub struct NextConfigurationPropertySource {
    properties: IndexMap<String, String>,
}

impl NextConfigurationPropertySource {
    /// Adapts the given property source into a configuration property source.
    ///
    /// Returns `None` when the source cannot be adapted, mirroring the nullable
    /// result of `ConfigurationPropertySource.from(PropertySource)`. Stub
    /// property sources are not adapted, since they only reserve a position and
    /// hold no properties.
    ///
    /// # Arguments
    ///
    /// * `source` - The property source to adapt.
    pub fn from(source: &dyn PropertySource<PropertySourceValue>) -> Option<Self> {
        if source.is_stub() {
            return None;
        }

        let properties = source
            .property_names()
            .into_iter()
            .filter_map(|name| source.property(&name).map(|value| (name, value)))
            .collect();

        Some(Self { properties })
    }

    /// Returns whether this source contains a property with the given name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the property.
    pub fn contains_property(&self, name: &str) -> bool {
        self.properties.contains_key(name)
    }

    /// Returns the names of the properties held by this source.
    pub fn property_names(&self) -> Vec<String> {
        self.properties.keys().cloned().collect()
    }
}

impl ConfigurationPropertySource for NextConfigurationPropertySource {
    /// Returns the value of the property with the given name, when present.
    fn get_configuration_property(
        &self,
        name: &ConfigurationPropertyName,
    ) -> Option<ConfigurationProperty> {
        self.properties
            .get(name.as_str())
            .map(|value| ConfigurationProperty::new(name.clone(), AnyValue::from(value.clone())))
    }
}

#[cfg(test)]
mod tests {
    use std::fmt;

    use super::*;
    use crate::env::MapPropertySource;

    /// A property source that only reserves a name, used to test stubs.
    struct StubSource {
        name: String,
    }

    impl PropertySource<PropertySourceValue> for StubSource {
        fn name(&self) -> &str {
            &self.name
        }

        fn property(&self, _name: &str) -> Option<String> {
            None
        }

        fn is_stub(&self) -> bool {
            true
        }

        fn source(&self) -> &PropertySourceValue {
            unreachable!("a stub holds no properties")
        }
    }

    impl fmt::Debug for StubSource {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "StubSource {{name='{}'}}", self.name)
        }
    }

    fn map_source(name: &str, pairs: &[(&str, &str)]) -> MapPropertySource {
        MapPropertySource::new(
            name.to_owned(),
            pairs
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect(),
        )
    }

    #[test]
    fn adapts_properties_of_the_underlying_source() {
        let source = map_source("test", &[("next.application.name", "demo")]);
        let adapted = NextConfigurationPropertySource::from(&source).unwrap();

        assert_eq!(adapted.property_names(), vec!["next.application.name"]);
        assert!(adapted.contains_property("next.application.name"));

        let name = ConfigurationPropertyName::of("next.application.name").unwrap();
        let property = adapted.get_configuration_property(&name).unwrap();

        assert_eq!(property.name(), &name);
        assert_eq!(property.get_value().as_str(), Some("demo"));
    }

    #[test]
    fn returns_none_for_unknown_property() {
        let source = map_source("test", &[("next.application.name", "demo")]);
        let adapted = NextConfigurationPropertySource::from(&source).unwrap();
        let name = ConfigurationPropertyName::of("next.application.version").unwrap();

        assert!(adapted.get_configuration_property(&name).is_none());
    }

    #[test]
    fn does_not_adapt_stub_sources() {
        let source = StubSource {
            name: "stub".to_owned(),
        };

        assert!(NextConfigurationPropertySource::from(&source).is_none());
    }
}
