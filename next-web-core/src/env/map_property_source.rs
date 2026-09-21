//! A [`PropertySource`] that is backed by a map of properties.

use crate::env::PropertySource;
use crate::util::indexmap::IndexMap;

/// A [`PropertySource`] holding its properties in a map.
///
/// The map preserves the order in which the properties were inserted, which
/// makes [`PropertySource::property_names`] deterministic for sources that are
/// built from a file, from the system environment or from any other ordered
/// collection.
#[derive(Debug, Clone)]
pub struct MapPropertySource {
    name: String,
    properties: IndexMap<String, String>,
}

impl MapPropertySource {
    /// Creates a new property source.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the property source.
    /// * `properties` - The properties of the source, in search order.
    pub fn new(name: String, properties: IndexMap<String, String>) -> Self {
        Self { name, properties }
    }

    /// Returns the properties of this source, in insertion order.
    pub fn properties(&self) -> &IndexMap<String, String> {
        &self.properties
    }
}

impl PropertySource<IndexMap<String, String>> for MapPropertySource {
    fn name(&self) -> &str {
        &self.name
    }

    fn property(&self, key: &str) -> Option<String> {
        self.properties.get(key).map(ToOwned::to_owned)
    }

    fn contains_property(&self, key: &str) -> bool {
        self.properties.contains_key(key)
    }

    fn property_names(&self) -> Vec<String> {
        self.properties.keys().cloned().collect()
    }

    fn source(&self) -> &IndexMap<String, String> {
        &self.properties
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a source with the given properties.
    fn source(pairs: &[(&str, &str)]) -> MapPropertySource {
        MapPropertySource::new(
            "test".to_owned(),
            pairs
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect(),
        )
    }

    #[test]
    fn exposes_its_properties() {
        let source = source(&[("a", "1"), ("b", "2")]);

        assert_eq!(source.name(), "test");
        assert_eq!(source.property("a"), Some("1".to_owned()));
        assert_eq!(source.property("missing"), None);
        assert!(source.contains_property("b"));
        assert!(!source.contains_property("missing"));
        assert_eq!(
            source.property_names(),
            vec!["a".to_owned(), "b".to_owned()]
        );
        assert_eq!(source.source().get("a"), Some(&"1".to_owned()));
    }

    #[test]
    fn keeps_the_insertion_order_of_its_properties() {
        let source = source(&[("second", "2"), ("first", "1")]);

        assert_eq!(
            source.property_names(),
            vec!["second".to_owned(), "first".to_owned()]
        );
    }
}
