//! A mutable, ordered collection of [`PropertySource`] instances.

use std::fmt;

use crate::env::PropertySource;
use crate::util::indexmap::IndexMap;

/// The value type exposed by property sources held in a
/// [`MutablePropertySources`].
pub type PropertySourceValue = IndexMap<String, String>;

/// A boxed property source suitable for storage in a
/// [`MutablePropertySources`].
pub type BoxedPropertySource = Box<dyn PropertySource<PropertySourceValue>>;

/// A mutable collection of [`PropertySource`] instances.
///
/// The collection is ordered by search precedence: the source at the front has
/// the highest precedence. Sources are identified by their
/// [`name`](PropertySource::name), so operations such as [`Self::get`],
/// [`Self::replace`], and [`Self::remove`] locate a source through its name.
#[derive(Default)]
pub struct MutablePropertySources {
    sources: Vec<BoxedPropertySource>,
}

impl MutablePropertySources {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds the given source with the highest precedence, at the front of the
    /// collection.
    pub fn add_first(&mut self, source: BoxedPropertySource) {
        self.sources.insert(0, source);
    }

    /// Adds the given source with the lowest precedence, at the back of the
    /// collection.
    pub fn add_last(&mut self, source: BoxedPropertySource) {
        self.sources.push(source);
    }

    /// Adds the given source immediately before the source with the given name.
    ///
    /// Does nothing when no source with the given name exists.
    pub fn add_before(&mut self, name: &str, source: BoxedPropertySource) {
        if let Some(index) = self.position(name) {
            self.sources.insert(index, source);
        }
    }

    /// Adds the given source immediately after the source with the given name.
    ///
    /// Does nothing when no source with the given name exists.
    pub fn add_after(&mut self, name: &str, source: BoxedPropertySource) {
        if let Some(index) = self.position(name) {
            self.sources.insert(index + 1, source);
        }
    }

    /// Returns whether a source with the given name exists.
    pub fn contains(&self, name: &str) -> bool {
        self.position(name).is_some()
    }

    /// Returns the source with the given name, if any.
    pub fn get(&self, name: &str) -> Option<&dyn PropertySource<PropertySourceValue>> {
        self.sources
            .iter()
            .find(|source| source.name() == name)
            .map(|source| source.as_ref())
    }

    /// Removes and returns the source with the given name, if any.
    pub fn remove(&mut self, name: &str) -> Option<BoxedPropertySource> {
        self.position(name).map(|index| self.sources.remove(index))
    }

    /// Replaces the source with the given name, returning the previous source.
    ///
    /// When no source with the given name exists, the source is appended to the
    /// end of the collection and `None` is returned.
    pub fn replace(
        &mut self,
        name: &str,
        source: BoxedPropertySource,
    ) -> Option<BoxedPropertySource> {
        match self.position(name) {
            Some(index) => Some(std::mem::replace(&mut self.sources[index], source)),
            None => {
                self.sources.push(source);
                None
            }
        }
    }

    /// Returns the number of sources in the collection.
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Returns whether the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Iterates over the sources from highest to lowest precedence.
    pub fn iter(&self) -> impl Iterator<Item = &dyn PropertySource<PropertySourceValue>> {
        self.sources.iter().map(|source| source.as_ref())
    }

    /// Returns the index of the source with the given name, if any.
    fn position(&self, name: &str) -> Option<usize> {
        self.sources.iter().position(|source| source.name() == name)
    }
}

impl fmt::Debug for MutablePropertySources {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.sources.iter().map(|source| source.name()))
            .finish()
    }
}
