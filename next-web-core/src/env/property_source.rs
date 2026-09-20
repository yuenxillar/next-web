use std::fmt;

use crate::env::BasePropertySource;

/// A source of name/value property pairs. The underlying source object may be
/// of any type `T` that encapsulates properties — for example, a map of
/// strings, a parsed configuration tree, or an in-memory handle.
///
/// `PropertySource` objects are not typically used in isolation, but rather
/// through a [`PropertySources`](crate::env::PropertySources) collection, which
/// aggregates property sources and, in conjunction with a
/// [`PropertyResolver`](crate::env::PropertyResolver), performs precedence-based
/// searches across the set.
///
/// **Identity is based on the [name](Self::name) alone**, not on the content of
/// the encapsulated properties. This is useful when manipulating property
/// sources in collection contexts (see `MutablePropertySources`), and it is why
/// [`PartialEq`] and [`Hash`] are expected to be implemented in terms of the
/// name only.
///
/// # Implementors
///
/// Implementations must provide at least [`name`](Self::name) and
/// [`property`](Self::property). The default [`contains_property`](Self::contains_property)
/// simply checks whether [`property`](Self::property) returns `Some`. Subclasses
/// may override it with a more efficient algorithm if possible.
pub trait PropertySource<T>
where
    Self: fmt::Debug,
{
    /// Return the name of this property source.
    ///
    /// The name determines the identity of the source when used in collections.
    fn name(&self) -> &str;

    /// Return the value associated with the given name, or `None` if not found.
    fn property(&self, name: &str) -> Option<String>;

    /// Return whether this property source contains the given name.
    ///
    /// The default implementation simply checks whether
    /// [`property`](Self::property) returns `Some`. Implementations may override
    /// this with a more efficient lookup if their backing store supports it.
    fn contains_property(&self, name: &str) -> bool {
        self.property(name).is_some()
    }

    /// Return the underlying source object, if the implementation exposes one.
    ///
    /// This is an escape hatch for callers that need to reach into the original
    /// data structure. It is not part of the property-lookup contract, so it is
    /// optional and may return `None` for implementations that do not carry a
    /// distinct source object.
    fn source(&self) -> &T;
}

/// Equality for property sources is based on the name alone, not on the
/// encapsulated properties. This mirrors the identity semantics described in
/// [`PropertySource`] and lets a source be located in a collection by name.
impl<T> PartialEq for dyn PropertySource<T> {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name()
    }
}

/// Hash codes are derived from the name only, consistent with equality.
impl<T> std::hash::Hash for dyn PropertySource<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name().hash(state);
    }
}

/// A placeholder property source that always reports the given name but holds
/// no properties.
///
/// Useful in cases where an actual property source cannot be eagerly
/// initialized at application startup. For example, a context-based property
/// source must wait until its underlying context object is available. In such
/// cases, a stub holds the intended default position/order of the property
/// source, then is replaced during context initialization.
pub struct StubPropertySource {
    base: BasePropertySource<()>,
}

impl StubPropertySource {
    /// Create a new stub with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            base: BasePropertySource::new(name, ()),
        }
    }
}

impl PropertySource<()> for StubPropertySource {
    fn name(&self) -> &str {
        self.base.name()
    }

    /// Always returns `None`.
    fn property(&self, _name: &str) -> Option<String> {
        None
    }

    fn source(&self) -> &() {
        self.base.source()
    }
}

impl fmt::Debug for StubPropertySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StubPropertySource {{name='{}'}}", self.name())
    }
}
