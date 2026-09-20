//! A collection of configuration property sources adapted from an environment.

use std::fmt;

use next_web_core::env::MutablePropertySources;

use crate::context::properties::source::{
    stream_property_sources, ConfigurationPropertySource, NextConfigurationPropertySource,
};

/// A collection of [`ConfigurationPropertySource`] instances adapted from the
/// property sources of an environment.
///
/// This is the Rust equivalent of Next Boot's
/// `NextConfigurationPropertySources`.
///
/// Note: Java iterates the underlying property sources lazily, so additions and
/// removals are tracked. Rust property sources are not shared, so this
/// collection snapshots them when it is created; use
/// [`is_using_sources`](Self::is_using_sources) to detect that it no longer
/// matches a given set of property sources.
pub struct NextConfigurationPropertySources {
    /// The names of the property sources this collection was adapted from.
    source_names: Vec<String>,
    /// The adapted sources, in search order.
    sources: Vec<Box<dyn ConfigurationPropertySource>>,
}

impl NextConfigurationPropertySources {
    /// Adapts the given property sources, skipping stub property sources and
    /// sources that cannot be adapted.
    ///
    /// # Arguments
    ///
    /// * `sources` - The property sources to adapt.
    pub fn new(sources: &MutablePropertySources) -> Self {
        let included = stream_property_sources(sources);
        let source_names = included
            .iter()
            .map(|source| source.name().to_owned())
            .collect();
        let sources = included
            .iter()
            .filter_map(|source| NextConfigurationPropertySource::from(*source))
            .map(|source| Box::new(source) as Box<dyn ConfigurationPropertySource>)
            .collect();

        Self {
            source_names,
            sources,
        }
    }

    /// Returns whether this collection was adapted from the given property
    /// sources.
    ///
    /// Note: Java compares the identity of the underlying iterable; this
    /// implementation compares the names of the adapted property sources.
    ///
    /// # Arguments
    ///
    /// * `sources` - The property sources to compare against.
    pub fn is_using_sources(&self, sources: &MutablePropertySources) -> bool {
        let source_names: Vec<String> = stream_property_sources(sources)
            .iter()
            .map(|source| source.name().to_owned())
            .collect();

        self.source_names == source_names
    }

    /// Returns the adapted sources, in search order.
    pub fn iter(&self) -> impl Iterator<Item = &dyn ConfigurationPropertySource> {
        self.sources.iter().map(|source| source.as_ref())
    }

    /// Returns the number of adapted sources.
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Returns whether this collection contains no adapted sources.
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}

impl fmt::Debug for NextConfigurationPropertySources {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NextConfigurationPropertySources")
            .field("source_names", &self.source_names)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::MapPropertySource;
    use next_web_core::util::indexmap::IndexMap;

    fn add_source(sources: &mut MutablePropertySources, name: &str, key: &str, value: &str) {
        let mut properties = IndexMap::new();
        properties.insert(key.to_owned(), value.to_owned());
        sources.add_last(Box::new(MapPropertySource::new(
            name.to_owned(),
            properties,
        )));
    }

    #[test]
    fn adapts_included_sources() {
        let mut sources = MutablePropertySources::new();
        add_source(&mut sources, "first", "next.application.name", "demo");
        add_source(
            &mut sources,
            "second",
            "next.application.version",
            "1.0.0",
        );

        let adapted = NextConfigurationPropertySources::new(&sources);

        assert_eq!(adapted.len(), 2);
        assert!(!adapted.is_empty());
        assert!(adapted.is_using_sources(&sources));
    }

    #[test]
    fn detects_that_the_sources_changed() {
        let mut sources = MutablePropertySources::new();
        add_source(&mut sources, "first", "next.application.name", "demo");

        let adapted = NextConfigurationPropertySources::new(&sources);
        assert!(adapted.is_using_sources(&sources));

        add_source(
            &mut sources,
            "second",
            "next.application.version",
            "1.0.0",
        );

        assert!(!adapted.is_using_sources(&sources));
    }

    #[test]
    fn adapts_sources_from_an_empty_collection() {
        let sources = MutablePropertySources::new();
        let adapted = NextConfigurationPropertySources::new(&sources);

        assert!(adapted.is_empty());
        assert_eq!(adapted.iter().count(), 0);
    }

    #[test]
    fn resolves_values_through_the_adapted_sources() {
        let mut sources = MutablePropertySources::new();
        add_source(&mut sources, "first", "next.application.name", "demo");

        let adapted = NextConfigurationPropertySources::new(&sources);
        let name = crate::context::properties::source::ConfigurationPropertyName::of(
            "next.application.name",
        )
        .unwrap();

        let value = adapted
            .iter()
            .find_map(|source| source.get_configuration_property(&name))
            .map(|property| property.get_value().to_string());

        assert_eq!(value, Some("demo".to_owned()));
    }
}
