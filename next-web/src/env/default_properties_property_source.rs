//! A [`PropertySource`] holding the default properties contributed directly to
//! a [`NextWebApplication`](crate::NextWebApplication).
//!
//! By convention, the [`DefaultPropertiesPropertySource`] is always the last
//! property source in the [`ConfigurableEnvironment`], so that its values have
//! the lowest precedence and never override an explicitly configured value.

use next_web_core::{
    env::{ConfigurableEnvironment, MutablePropertySources, PropertySource},
    util::indexmap::IndexMap,
};

use crate::env::MapPropertySource;

/// The name of the `defaultProperties` property source.
pub const DEFAULT_PROPERTIES_PROPERTY_SOURCE_NAME: &str = "defaultProperties";

/// A map-backed [`PropertySource`] containing default properties contributed
/// directly to a [`NextWebApplication`](crate::NextWebApplication).
#[derive(Debug)]
pub struct DefaultPropertiesPropertySource {
    delegate: MapPropertySource,
}

impl DefaultPropertiesPropertySource {
    /// The name of the `defaultProperties` property source.
    pub const NAME: &'static str = DEFAULT_PROPERTIES_PROPERTY_SOURCE_NAME;

    /// Creates a new source backed by the given properties.
    ///
    /// # Arguments
    ///
    /// * `source` - The default properties, in search order.
    pub fn new(source: IndexMap<String, String>) -> Self {
        Self {
            delegate: MapPropertySource::new(Self::NAME.to_string(), source),
        }
    }

    /// Returns `true` if the given source is named `defaultProperties`.
    ///
    /// # Arguments
    ///
    /// * `property_source` - The property source to check.
    pub fn has_matching_name<T>(property_source: Option<&dyn PropertySource<T>>) -> bool {
        property_source
            .map(|source| source.name() == Self::NAME)
            .unwrap_or(false)
    }

    /// Creates a new [`DefaultPropertiesPropertySource`] if the provided
    /// properties are not empty, and passes it to the given action.
    ///
    /// # Arguments
    ///
    /// * `source` - The default properties, in search order.
    /// * `action` - The action invoked with the new source when `source` is not
    ///   empty.
    pub fn if_not_empty<F>(source: IndexMap<String, String>, action: F)
    where
        F: FnOnce(DefaultPropertiesPropertySource),
    {
        if !source.is_empty() {
            action(Self::new(source));
        }
    }

    /// Adds a new [`DefaultPropertiesPropertySource`], or merges it with an
    /// existing one.
    ///
    /// When a source named `defaultProperties` already exists, its properties
    /// are merged with `source` (existing values first, `source` winning) and
    /// the source keeps its current position. Otherwise a new source is added
    /// with the lowest precedence.
    ///
    /// # Arguments
    ///
    /// * `source` - The default properties to add or merge.
    /// * `sources` - The collection of property sources to update.
    pub fn add_or_merge(source: IndexMap<String, String>, sources: &mut MutablePropertySources) {
        if source.is_empty() {
            return;
        }

        if sources.contains(Self::NAME) {
            let mut resulting = IndexMap::new();
            if let Some(existing) = sources.get(Self::NAME) {
                resulting.extend(
                    existing
                        .source()
                        .iter()
                        .map(|(key, value)| (key.clone(), value.clone())),
                );
            }
            resulting.extend(source);

            sources.replace(Self::NAME, Box::new(Self::new(resulting)));
        } else {
            sources.add_last(Box::new(Self::new(source)));
        }
    }

    /// Moves the `defaultProperties` source so that it is the last source in
    /// the given [`ConfigurableEnvironment`].
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment to update.
    pub fn move_to_end(environment: &mut dyn ConfigurableEnvironment) {
        Self::move_to_end_sources(environment.property_sources());
    }

    /// Moves the `defaultProperties` source so that it is the last source in
    /// the given [`MutablePropertySources`].
    ///
    /// # Arguments
    ///
    /// * `sources` - The property sources to update.
    pub fn move_to_end_sources(sources: &mut MutablePropertySources) {
        if let Some(source) = sources.remove(Self::NAME) {
            sources.add_last(source);
        }
    }
}

impl PropertySource<IndexMap<String, String>> for DefaultPropertiesPropertySource {
    fn name(&self) -> &str {
        self.delegate.name()
    }

    fn property(&self, key: &str) -> Option<String> {
        self.delegate.property(key)
    }

    fn contains_property(&self, key: &str) -> bool {
        self.delegate.contains_property(key)
    }

    fn property_names(&self) -> Vec<String> {
        self.delegate.property_names()
    }

    fn source(&self) -> &IndexMap<String, String> {
        self.delegate.source()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn properties(pairs: &[(&str, &str)]) -> IndexMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn exposes_default_properties_name() {
        let source = DefaultPropertiesPropertySource::new(properties(&[("a", "1")]));

        assert_eq!(source.name(), DEFAULT_PROPERTIES_PROPERTY_SOURCE_NAME);
        assert_eq!(source.property("a"), Some("1".to_string()));
        assert!(source.contains_property("a"));
        assert_eq!(source.source().get("a"), Some(&"1".to_string()));
    }

    #[test]
    fn matches_only_default_properties_name() {
        let matching = DefaultPropertiesPropertySource::new(IndexMap::new());
        let other = MapPropertySource::new("other".to_string(), IndexMap::new());

        assert!(DefaultPropertiesPropertySource::has_matching_name(Some(
            &matching
        )));
        assert!(!DefaultPropertiesPropertySource::has_matching_name(Some(
            &other
        )));
        assert!(!DefaultPropertiesPropertySource::has_matching_name::<
            IndexMap<String, String>,
        >(None));
    }

    #[test]
    fn if_not_empty_skips_empty_sources() {
        let mut called = false;
        DefaultPropertiesPropertySource::if_not_empty(IndexMap::new(), |_| called = true);
        assert!(!called);

        DefaultPropertiesPropertySource::if_not_empty(properties(&[("a", "1")]), |source| {
            called = true;
            assert_eq!(source.name(), DEFAULT_PROPERTIES_PROPERTY_SOURCE_NAME);
        });
        assert!(called);
    }

    #[test]
    fn add_or_merge_appends_then_merges() {
        let mut sources = MutablePropertySources::new();

        DefaultPropertiesPropertySource::add_or_merge(properties(&[("a", "1")]), &mut sources);
        assert_eq!(sources.len(), 1);
        assert_eq!(
            sources
                .get(DefaultPropertiesPropertySource::NAME)
                .unwrap()
                .property("a"),
            Some("1".to_string())
        );

        DefaultPropertiesPropertySource::add_or_merge(
            properties(&[("a", "2"), ("b", "3")]),
            &mut sources,
        );

        assert_eq!(sources.len(), 1);
        let merged = sources.get(DefaultPropertiesPropertySource::NAME).unwrap();
        assert_eq!(merged.property("a"), Some("2".to_string()));
        assert_eq!(merged.property("b"), Some("3".to_string()));
    }

    #[test]
    fn add_or_merge_ignores_empty_source() {
        let mut sources = MutablePropertySources::new();

        DefaultPropertiesPropertySource::add_or_merge(IndexMap::new(), &mut sources);

        assert!(sources.is_empty());
    }

    #[test]
    fn move_to_end_sources_reorders_last() {
        let mut sources = MutablePropertySources::new();
        sources.add_first(Box::new(DefaultPropertiesPropertySource::new(properties(
            &[("a", "1")],
        ))));
        sources.add_last(Box::new(MapPropertySource::new(
            "other".to_string(),
            IndexMap::new(),
        )));

        assert_eq!(
            sources.iter().next().map(|source| source.name()),
            Some(DefaultPropertiesPropertySource::NAME)
        );

        DefaultPropertiesPropertySource::move_to_end_sources(&mut sources);

        assert_eq!(
            sources.iter().last().map(|source| source.name()),
            Some(DefaultPropertiesPropertySource::NAME)
        );
    }
}
