use std::fmt;

use crate::env::PropertySource;

pub struct BasePropertySource<T> {
    name: String,
    source: T,
}

impl<T> BasePropertySource<T> {
    /// Create a new PropertySource with the given name and source object.
    pub fn new(name: impl Into<String>, source: T) -> Self {
        Self {
            name: name.into(),
            source,
        }
    }

    /// Return the name of this PropertySource.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the underlying source object for this PropertySource.
    pub fn source(&self) -> &T {
        &self.source
    }

    /// Return a [`PropertySource`] intended for collection comparison purposes only.
    ///
    /// Primarily for internal use, but given a collection of property sources, it
    /// may be used as follows:
    ///
    /// ```ignore
    /// let mut sources: Vec<Box<dyn PropertySource>> = Vec::new();
    /// sources.push(Box::new(MapPropertySource::new("sourceA", map_a)));
    /// sources.push(Box::new(MapPropertySource::new("sourceB", map_b)));
    /// assert!(sources.iter().any(|s| s.name() == comparison_source("sourceA").name()));
    /// assert!(sources.iter().any(|s| s.name() == comparison_source("sourceB").name()));
    /// assert!(!sources.iter().any(|s| s.name() == comparison_source("sourceC").name()));
    /// ```
    ///
    /// The returned source panics if any methods other than equality, hashing, and
    /// formatting are called.
    pub fn named(name: impl Into<String>) -> impl PropertySource<()> {
        ComparisonPropertySource::new(name)
    }
}

impl<T> Clone for BasePropertySource<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            source: self.source.clone(),
        }
    }
}

/// A property source intended for collection comparison purposes only.
///
/// The name is used for equality and hashing, but any attempt to query the
/// source's contents panics, since the instance carries no real data. Obtain
/// one through [`comparison_source`].
///
/// # Panics
///
/// Panics if [`source`](PropertySource::source),
/// [`contains_property`](PropertySource::contains_property), or
/// [`property`](PropertySource::property) are called.
pub struct ComparisonPropertySource {
    name: String,
}

const USAGE_ERROR: &str =
    "ComparisonPropertySource instances are for use with collection comparison only";

impl ComparisonPropertySource {
    /// Create a new comparison source with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl PropertySource<()> for ComparisonPropertySource {
    fn name(&self) -> &str {
        &self.name
    }

    fn source(&self) -> &() {
        panic!("{}", USAGE_ERROR);
    }

    fn contains_property(&self, _name: &str) -> bool {
        panic!("{}", USAGE_ERROR);
    }

    fn property(&self, _name: &str) -> Option<String> {
        panic!("{}", USAGE_ERROR);
    }
}

impl fmt::Debug for ComparisonPropertySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ComparisonPropertySource {{name='{}'}}", self.name)
    }
}
