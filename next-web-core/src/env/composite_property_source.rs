//! A [`PropertySource`] that delegates to a set of contained property sources.

use std::fmt;

use crate::env::PropertySource;
use crate::util::indexmap::IndexMap;

/// A composite [`PropertySource`] that iterates over a set of contained
/// [`PropertySource`] instances.
///
/// This is useful when multiple sources share the same name, for example when
/// several values are contributed to a single logical source. Lookups return
/// the value of the first contained source that has the property, in insertion
/// order.
///
/// The contained sources are identified by
/// [`name`](PropertySource::name), mirroring Spring's `LinkedHashSet`
/// semantics: adding a source whose name is already present is a no-op, and
/// adding it first moves it ahead of the existing source with the same name.
///
/// # Type parameters
///
/// * `T` - the value type exposed by the contained sources, defaulting to
///   [`IndexMap<String, String>`](IndexMap), which is the representation used
///   by the framework's map-backed sources.
pub struct CompositePropertySource<T = IndexMap<String, String>> {
    name: String,
    source: T,
    property_sources: Vec<Box<dyn PropertySource<T>>>,
}

impl<T> CompositePropertySource<T>
where
    T: Default,
{
    /// Creates a new composite source with the given name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the property source.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: T::default(),
            property_sources: Vec::new(),
        }
    }
}

impl<T> CompositePropertySource<T> {
    /// Adds the given source to the end of the chain.
    ///
    /// If a source with the same name is already present, it is left unchanged.
    ///
    /// # Arguments
    ///
    /// * `property_source` - The property source to add.
    pub fn add_property_source(&mut self, property_source: Box<dyn PropertySource<T>>) {
        let name = property_source.name().to_string();
        if self
            .property_sources
            .iter()
            .any(|source| source.name() == name)
        {
            return;
        }
        self.property_sources.push(property_source);
    }

    /// Adds the given source to the start of the chain.
    ///
    /// Any existing source with the same name is removed first, so the new
    /// source takes precedence.
    ///
    /// # Arguments
    ///
    /// * `property_source` - The property source to add.
    pub fn add_first_property_source(&mut self, property_source: Box<dyn PropertySource<T>>) {
        let name = property_source.name().to_string();
        self.property_sources.retain(|source| source.name() != name);
        self.property_sources.insert(0, property_source);
    }

    /// Returns all property sources held by this composite, in lookup order.
    pub fn property_sources(&self) -> &[Box<dyn PropertySource<T>>] {
        &self.property_sources
    }

    /// Returns the number of contained property sources.
    pub fn len(&self) -> usize {
        self.property_sources.len()
    }

    /// Returns whether this composite contains no property sources.
    pub fn is_empty(&self) -> bool {
        self.property_sources.is_empty()
    }
}

impl<T> PropertySource<T> for CompositePropertySource<T> {
    fn name(&self) -> &str {
        &self.name
    }

    /// Returns the value of the first contained source that has the property.
    fn property(&self, name: &str) -> Option<String> {
        self.property_sources
            .iter()
            .find_map(|source| source.property(name))
    }

    /// Returns whether any contained source has the property.
    fn contains_property(&self, name: &str) -> bool {
        self.property_sources
            .iter()
            .any(|source| source.contains_property(name))
    }

    /// Returns the accumulated property names from all contained sources,
    /// de-duplicated while preserving first-seen order.
    fn property_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for source in &self.property_sources {
            for name in source.property_names() {
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }
        names
    }

    /// Returns the composite's placeholder source object.
    ///
    /// A composite has no single backing store, so the default value of `T` is
    /// exposed here.
    fn source(&self) -> &T {
        &self.source
    }
}

impl<T> fmt::Debug for CompositePropertySource<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<&str> = self
            .property_sources
            .iter()
            .map(|source| source.name())
            .collect();

        f.debug_struct("CompositePropertySource")
            .field("name", &self.name)
            .field("property_sources", &names)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestPropertySource {
        name: String,
        properties: IndexMap<String, String>,
    }

    impl TestPropertySource {
        fn new(name: &str, pairs: &[(&str, &str)]) -> Self {
            Self {
                name: name.to_string(),
                properties: pairs
                    .iter()
                    .map(|(key, value)| (key.to_string(), value.to_string()))
                    .collect(),
            }
        }
    }

    impl PropertySource<IndexMap<String, String>> for TestPropertySource {
        fn name(&self) -> &str {
            &self.name
        }

        fn property(&self, name: &str) -> Option<String> {
            self.properties.get(name).cloned()
        }

        fn contains_property(&self, name: &str) -> bool {
            self.properties.contains_key(name)
        }

        fn property_names(&self) -> Vec<String> {
            self.properties.keys().cloned().collect()
        }

        fn source(&self) -> &IndexMap<String, String> {
            &self.properties
        }
    }

    fn source(
        name: &str,
        pairs: &[(&str, &str)],
    ) -> Box<dyn PropertySource<IndexMap<String, String>>> {
        Box::new(TestPropertySource::new(name, pairs))
    }

    #[test]
    fn returns_first_matching_property() {
        let mut composite = CompositePropertySource::new("composite");
        composite.add_property_source(source("first", &[("a", "1"), ("b", "1")]));
        composite.add_property_source(source("second", &[("b", "2"), ("c", "2")]));

        assert_eq!(composite.property("a"), Some("1".to_string()));
        assert_eq!(composite.property("b"), Some("1".to_string()));
        assert_eq!(composite.property("c"), Some("2".to_string()));
        assert_eq!(composite.property("missing"), None);
    }

    #[test]
    fn contains_property_checks_all_sources() {
        let mut composite = CompositePropertySource::new("composite");
        composite.add_property_source(source("first", &[("a", "1")]));
        composite.add_property_source(source("second", &[("b", "2")]));

        assert!(composite.contains_property("a"));
        assert!(composite.contains_property("b"));
        assert!(!composite.contains_property("c"));
    }

    #[test]
    fn property_names_are_accumulated_and_deduplicated() {
        let mut composite = CompositePropertySource::new("composite");
        composite.add_property_source(source("first", &[("a", "1"), ("b", "1")]));
        composite.add_property_source(source("second", &[("b", "2"), ("c", "2")]));

        assert_eq!(composite.property_names(), vec!["a", "b", "c"]);
    }

    #[test]
    fn duplicate_names_are_not_added_twice() {
        let mut composite = CompositePropertySource::new("composite");
        composite.add_property_source(source("same", &[("a", "1")]));
        composite.add_property_source(source("same", &[("b", "2")]));

        assert_eq!(composite.len(), 1);
        assert_eq!(composite.property("a"), Some("1".to_string()));
        assert_eq!(composite.property("b"), None);
    }

    #[test]
    fn add_first_moves_source_ahead() {
        let mut composite = CompositePropertySource::new("composite");
        composite.add_property_source(source("first", &[("a", "1")]));
        composite.add_first_property_source(source("second", &[("a", "2")]));

        assert_eq!(composite.len(), 2);
        assert_eq!(composite.property("a"), Some("2".to_string()));

        // Re-adding a source with an existing name at the front replaces it.
        composite.add_first_property_source(source("first", &[("a", "3")]));
        assert_eq!(composite.len(), 2);
        assert_eq!(composite.property("a"), Some("3".to_string()));
    }

    #[test]
    fn exposes_name_and_contained_sources() {
        let mut composite = CompositePropertySource::new("composite");
        composite.add_property_source(source("first", &[("a", "1")]));

        assert_eq!(composite.name(), "composite");
        assert_eq!(composite.property_sources().len(), 1);
        assert_eq!(composite.property_sources()[0].name(), "first");
        assert!(!composite.is_empty());
    }
}
