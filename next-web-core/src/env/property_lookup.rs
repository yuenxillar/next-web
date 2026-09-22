//! The strategy an environment uses to look the value of a property up.

use crate::env::MutablePropertySources;

/// Strategy used by an [`Environment`](crate::env::Environment) to look up the
/// value of a property.
///
/// The property sources of the environment are passed in on every call,
/// because an environment owns its property sources and a lookup therefore
/// cannot hold a reference to them.
///
/// Values are looked up as they are stored: resolving the placeholders of a
/// value, and reporting a property that is missing as an error, is the
/// responsibility of the environment that uses the strategy.
pub trait PropertyLookup
where
    Self: Send + Sync,
{
    /// Returns the value of the given key, or `None` when none of the property
    /// sources has it.
    ///
    /// # Arguments
    ///
    /// * `property_sources` - The property sources to look the key up in.
    /// * `key` - The name of the property.
    fn lookup(&self, property_sources: &MutablePropertySources, key: &str) -> Option<String>;
}

/// The [`PropertyLookup`] that is used by default.
///
/// The property sources are searched in the order in which they are held by the
/// environment, so the first source that has the property wins. Property
/// sources that hold no properties, such as the place holders that reserve the
/// position of a source that is created later, are skipped by the lookup of the
/// source itself.
#[derive(Debug, Clone, Copy, Default)]
pub struct PropertySourcesLookup;

impl PropertyLookup for PropertySourcesLookup {
    fn lookup(&self, property_sources: &MutablePropertySources, key: &str) -> Option<String> {
        property_sources
            .iter()
            .find_map(|property_source| property_source.property(key))
    }
}

#[cfg(test)]
mod tests {
    use crate::env::{MapPropertySource, MutablePropertySources};
    use crate::util::indexmap::IndexMap;

    use super::*;

    /// Adds a map backed property source with the given properties.
    fn add_source(sources: &mut MutablePropertySources, name: &str, properties: &[(&str, &str)]) {
        let properties: IndexMap<String, String> = properties
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        sources.add_last(Box::new(MapPropertySource::new(
            name.to_owned(),
            properties,
        )));
    }

    #[test]
    fn returns_the_value_of_the_first_source_that_has_the_property() {
        let mut sources = MutablePropertySources::new();
        add_source(&mut sources, "first", &[("a", "1"), ("b", "1")]);
        add_source(&mut sources, "second", &[("b", "2"), ("c", "2")]);

        let lookup = PropertySourcesLookup;

        assert_eq!(lookup.lookup(&sources, "a"), Some("1".to_owned()));
        assert_eq!(lookup.lookup(&sources, "b"), Some("1".to_owned()));
        assert_eq!(lookup.lookup(&sources, "c"), Some("2".to_owned()));
    }

    #[test]
    fn returns_none_for_a_property_that_is_absent() {
        let mut sources = MutablePropertySources::new();
        add_source(&mut sources, "first", &[("a", "1")]);

        assert_eq!(PropertySourcesLookup.lookup(&sources, "missing"), None);
        assert_eq!(
            PropertySourcesLookup.lookup(&MutablePropertySources::new(), "a"),
            None
        );
    }
}
