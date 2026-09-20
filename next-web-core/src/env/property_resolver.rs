use crate::{convert::ConversionError, error::IllegalError};

/// Trait for resolving properties against any underlying source.
///
/// A property resolver maps string keys to string values, optionally
/// converting values to a requested target type and resolving `${...}`
/// placeholders embedded in text.
///
/// See [`Environment`](crate::env::Environment) and
/// [`PropertySourcesPropertyResolver`](crate::env::PropertySourcesPropertyResolver).
pub trait PropertyResolver {
    /// Determine whether the given property key is available for resolution
    /// — for example, if the value for the given key is not `None`.
    fn contains_property(&self, key: &str) -> bool;

    /// Resolve the property value associated with the given key,
    /// or `None` if the key cannot be resolved.
    ///
    /// See [`get_property_or`](Self::get_property_or),
    /// [`get_property_as`](Self::get_property_as),
    /// [`get_required_property`](Self::get_required_property).
    fn get_property(&self, key: &str) -> Option<String>;

    /// Resolve the property value associated with the given key, or
    /// `default_value` if the key cannot be resolved.
    ///
    /// See [`get_property`](Self::get_property),
    /// [`get_required_property`](Self::get_required_property).
    fn get_property_or_default(&self, key: &str, default_value: &str) -> &str;

    // /// Resolve the property value associated with the given key, converted
    // /// to the requested target type, or `None` if the key cannot be resolved.
    // ///
    // /// Returns an error if the value cannot be converted to `T`.
    // ///
    // /// See [`get_required_property_as`](Self::get_required_property_as).
    // fn get_property_as<T>(&self, key: &str) -> Result<Option<T>, ConversionError>;

    // /// Resolve the property value associated with the given key, converted
    // /// to the requested target type, or `default_value` if the key cannot
    // /// be resolved.
    // ///
    // /// Returns an error if the value cannot be converted to `T`.
    // ///
    // /// See [`get_required_property_as`](Self::get_required_property_as).
    // fn get_property_as_or_default<T>(
    //     &self,
    //     key: &str,
    //     default_value: T,
    // ) -> Result<T, ConversionError>;

    /// Resolve the property value associated with the given key.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError`] if the key cannot be resolved.
    ///
    /// See [`get_required_property_as`](Self::get_required_property_as).
    fn get_required_property(&self, key: &str) -> Result<String, IllegalError>;

    // /// Resolve the property value associated with the given key, converted
    // /// to the requested target type.
    // ///
    // /// # Errors
    // ///
    // /// Returns [`ResolveError`] if the key cannot be resolved, or
    // /// [`ConversionError`] if the value cannot be converted to `T`.
    // fn get_required_property_as<T>(&self, key: &str) -> Result<T, IllegalError>;

    /// Resolve `${...}` placeholders in the given text, replacing them with
    /// corresponding property values as resolved by
    /// [`get_property`](Self::get_property).
    ///
    /// Unresolvable placeholders with no default value are ignored and passed
    /// through unchanged.
    ///
    /// # Panics
    ///
    /// Implementations may panic if `text` is empty in ways they cannot
    /// represent. Callers should not pass `none`.
    fn resolve_placeholders(&self, text: &str) -> String;

    /// Resolve `${...}` placeholders in the given text, replacing them with
    /// corresponding property values as resolved by
    /// [`get_property`](Self::get_property).
    ///
    /// # Errors
    ///
    /// Returns [`PlaceholderError`] if any placeholder cannot be resolved.
    fn resolve_required_placeholders(&self, text: &str) -> Result<String, IllegalError>;
}
