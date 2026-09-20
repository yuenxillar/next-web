//! Optional strategy used by a [`Binder`] to resolve property placeholders.

use next_web_core::anys::any_value::AnyValue;

/// Optional strategy that used by a [`Binder`] to resolve property placeholders.
///
/// This is a functional interface; in Rust it is represented as a trait object
/// ([`Box<dyn PlaceholdersResolver>`]) or a generic type parameter with a trait bound.
///
/// See [`PropertySourcesPlaceholdersResolver`] for a common implementation.
pub trait PlaceholdersResolver {
    /// Called to resolve any placeholders in the given value.
    ///
    /// # Arguments
    ///
    /// * `value` - the source value
    ///
    /// # Returns
    ///
    /// A value with placeholders resolved, or `None` if the input was `None`.
    fn resolve_placeholders(&self, value: Option<AnyValue>) -> Option<AnyValue>;
}

/// A no-op [`PlaceholdersResolver`] that returns the input value unchanged.
///
/// Corresponds to `PlaceholdersResolver.NONE` in the Java version.
pub struct NoOpPlaceholdersResolver;

impl PlaceholdersResolver for NoOpPlaceholdersResolver {
    fn resolve_placeholders(&self, value: Option<AnyValue>) -> Option<AnyValue> {
        value
    }
}
