//! Utility for working with strings that have placeholder values in them.
//!
//! A placeholder takes the form `${name}`. Using [`PropertyPlaceholderHelper`] these
//! placeholders can be substituted for user-supplied values.
//!
//! Values for substitution can be supplied using a property map or using a
//! [`PlaceholderResolver`].

use std::collections::HashMap;
use std::sync::Arc;

use crate::PlaceholderParser;
use crate::error::PlaceholderResolutionError;

/// Strategy interface used to resolve replacement values for placeholders contained in
/// strings.
///
/// Corresponds to the Java `PlaceholderResolver` functional interface.
pub trait PlaceholderResolver: Send + Sync {
    /// Resolve the supplied placeholder name to the replacement value.
    ///
    /// # Arguments
    ///
    /// * `placeholder_name` - the name of the placeholder to resolve
    ///
    /// # Returns
    ///
    /// The replacement value, or `None` if no replacement is to be made.
    fn resolve_placeholder(&self, placeholder_name: &str) -> Option<String>;
}

impl<F> PlaceholderResolver for F
where
    F: Fn(&str) -> Option<String> + Send + Sync,
{
    fn resolve_placeholder(&self, placeholder_name: &str) -> Option<String> {
        self(placeholder_name)
    }
}

/// A [`PlaceholderResolver`] backed by a property map.
///
/// Mirrors the Java `Properties::getProperty` method reference usage.
pub struct MapPlaceholderResolver {
    properties: HashMap<String, String>,
}

impl MapPlaceholderResolver {
    /// Creates a new resolver backed by the given property map.
    pub fn new(properties: HashMap<String, String>) -> Self {
        Self { properties }
    }

    /// Creates a new resolver from a borrowed map by cloning its entries.
    pub fn from_ref(properties: &HashMap<String, String>) -> Self {
        Self {
            properties: properties.clone(),
        }
    }
}

impl PlaceholderResolver for MapPlaceholderResolver {
    fn resolve_placeholder(&self, placeholder_name: &str) -> Option<String> {
        self.properties.get(placeholder_name).cloned()
    }
}

/// Utility class for working with strings that have placeholder values in them.
///
/// A placeholder takes the form `${name}`. Using [`PropertyPlaceholderHelper`] these
/// placeholders can be substituted for user-supplied values.
pub struct PropertyPlaceholderHelper {
    parser: PlaceholderParser,
}

impl PropertyPlaceholderHelper {
    /// Creates a new [`PropertyPlaceholderHelper`] that uses the supplied prefix and
    /// suffix.
    ///
    /// Unresolvable placeholders are ignored.
    ///
    /// # Arguments
    ///
    /// * `placeholder_prefix` - the prefix that denotes the start of a placeholder
    /// * `placeholder_suffix` - the suffix that denotes the end of a placeholder
    ///
    /// # Errors
    ///
    /// Returns a [`PlaceholderResolutionError`] if either prefix or suffix is
    /// empty (mirrors the Java `Assert.notNull` checks, adapted to Rust's
    /// non-nullable strings).
    pub fn new(
        placeholder_prefix: String,
        placeholder_suffix: String,
    ) -> Result<Self, PlaceholderResolutionError> {
        Self::with_options(placeholder_prefix, placeholder_suffix, None, None, true)
    }

    /// Creates a new [`PropertyPlaceholderHelper`] that uses the supplied prefix and
    /// suffix.
    ///
    /// # Arguments
    ///
    /// * `placeholder_prefix` - the prefix that denotes the start of a placeholder
    /// * `placeholder_suffix` - the suffix that denotes the end of a placeholder
    /// * `value_separator` - the separating character between the placeholder variable
    ///   and the associated default value, if any
    /// * `escape_character` - the escape character to use to ignore placeholder prefix
    ///   or value separator, if any
    /// * `ignore_unresolvable_placeholders` - whether unresolvable placeholders should
    ///   be ignored (`true`) or cause an error (`false`)
    pub fn with_options(
        placeholder_prefix: String,
        placeholder_suffix: String,
        value_separator: Option<String>,
        escape_character: Option<char>,
        ignore_unresolvable_placeholders: bool,
    ) -> Result<Self, PlaceholderResolutionError> {
        if placeholder_prefix.is_empty() {
            return Err(PlaceholderResolutionError::with_message(
                "'placeholderPrefix' must not be empty",
            ));
        }
        if placeholder_suffix.is_empty() {
            return Err(PlaceholderResolutionError::with_message(
                "'placeholderSuffix' must not be empty",
            ));
        }
        tracing::debug!(
            prefix = %placeholder_prefix,
            suffix = %placeholder_suffix,
            "creating PropertyPlaceholderHelper"
        );
        Ok(Self {
            parser: PlaceholderParser::new(
                placeholder_prefix,
                placeholder_suffix,
                value_separator,
                escape_character,
                ignore_unresolvable_placeholders,
            ),
        })
    }

    /// Replace all placeholders of format `${name}` with the corresponding property
    /// from the supplied property map.
    ///
    /// # Arguments
    ///
    /// * `value` - the value containing the placeholders to be replaced
    /// * `properties` - the properties to use for replacement
    ///
    /// # Returns
    ///
    /// The supplied value with placeholders replaced inline.
    pub fn replace_placeholders_with_map(
        &self,
        value: &str,
        properties: &HashMap<String, String>,
    ) -> Result<String, PlaceholderResolutionError> {
        let resolver = MapPlaceholderResolver::from_ref(properties);
        self.replace_placeholders(value, &resolver)
    }

    /// Replace all placeholders of format `${name}` with the value returned from the
    /// supplied [`PlaceholderResolver`].
    ///
    /// # Arguments
    ///
    /// * `value` - the value containing the placeholders to be replaced
    /// * `placeholder_resolver` - the resolver to use for replacement
    ///
    /// # Returns
    ///
    /// The supplied value with placeholders replaced inline.
    pub fn replace_placeholders(
        &self,
        value: &str,
        placeholder_resolver: &dyn PlaceholderResolver,
    ) -> Result<String, PlaceholderResolutionError> {
        tracing::trace!(value, "replacing placeholders");
        self.parser
            .replace_placeholders(value, placeholder_resolver)
    }

    /// Returns a shared reference to the underlying parser.
    pub fn parser(&self) -> &PlaceholderParser {
        &self.parser
    }
}

/// A convenience wrapper allowing a [`PropertyPlaceholderHelper`] to be shared across
/// threads without cloning the parser.
pub type SharedPropertyPlaceholderHelper = Arc<PropertyPlaceholderHelper>;

#[cfg(test)]
mod tests {
    use super::*;

    fn helper() -> PropertyPlaceholderHelper {
        PropertyPlaceholderHelper::with_options(
            "${".to_owned(),
            "}".to_owned(),
            Some(":".to_owned()),
            Some('\\'),
            true,
        )
        .unwrap()
    }

    #[test]
    fn new_rejects_empty_prefix() {
        let result = PropertyPlaceholderHelper::new(String::new(), "}".to_owned());
        assert!(result.is_err());
    }

    #[test]
    fn new_rejects_empty_suffix() {
        let result = PropertyPlaceholderHelper::new("${".to_owned(), String::new());
        assert!(result.is_err());
    }

    #[test]
    fn replaces_from_map() {
        let mut props = HashMap::new();
        props.insert("name".to_owned(), "World".to_owned());
        let result = helper()
            .replace_placeholders_with_map("Hello ${name}", &props)
            .unwrap();
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn replaces_with_custom_resolver() {
        let resolver = |name: &str| -> Option<String> {
            if name == "name" {
                Some("Rust".to_owned())
            } else {
                None
            }
        };
        let result = helper()
            .replace_placeholders("Hello ${name}", &resolver)
            .unwrap();
        assert_eq!(result, "Hello Rust");
    }

    #[test]
    fn keeps_unresolvable_when_ignored() {
        let props = HashMap::new();
        let result = helper()
            .replace_placeholders_with_map("Hello ${missing}", &props)
            .unwrap();
        assert_eq!(result, "Hello ${missing}");
    }

    #[test]
    fn errors_on_unresolvable_when_strict() {
        let strict = PropertyPlaceholderHelper::with_options(
            "${".to_owned(),
            "}".to_owned(),
            Some(":".to_owned()),
            Some('\\'),
            false,
        )
        .unwrap();
        let props = HashMap::new();
        let err = strict
            .replace_placeholders_with_map("Hello ${missing}", &props)
            .unwrap_err();
        assert!(err.message().contains("Could not resolve placeholder"));
    }

    #[test]
    fn uses_default_value_from_separator() {
        let props = HashMap::new();
        let result = helper()
            .replace_placeholders_with_map("Hello ${name:John}", &props)
            .unwrap();
        assert_eq!(result, "Hello John");
    }

    #[test]
    fn escapes_placeholder() {
        let props = HashMap::new();
        let result = helper()
            .replace_placeholders_with_map(r"\${name}", &props)
            .unwrap();
        assert_eq!(result, "${name}");
    }

    #[test]
    fn shared_helper_can_be_cloned_across_threads() {
        let helper: SharedPropertyPlaceholderHelper = Arc::new(helper());
        let clone = Arc::clone(&helper);
        let handle = std::thread::spawn(move || {
            let mut props = HashMap::new();
            props.insert("name".to_owned(), "Thread".to_owned());
            clone
                .replace_placeholders_with_map("Hi ${name}", &props)
                .unwrap()
        });
        assert_eq!(handle.join().unwrap(), "Hi Thread");
    }
}
