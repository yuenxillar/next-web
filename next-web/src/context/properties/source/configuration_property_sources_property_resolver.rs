//! A property resolver that resolves values against configuration property
//! sources.

use next_web_core::error::IllegalError;

use crate::context::properties::source::{
    ConfigurationPropertyName, NextConfigurationPropertySources,
};

/// The placeholder prefix used by default.
const DEFAULT_PLACEHOLDER_PREFIX: &str = "${";

/// The placeholder suffix used by default.
const DEFAULT_PLACEHOLDER_SUFFIX: &str = "}";

/// Separates the property name from the default value of a placeholder.
const DEFAULT_VALUE_SEPARATOR: char = ':';

/// Maximum number of nested placeholders that are resolved.
const MAX_PLACEHOLDER_DEPTH: usize = 8;

/// A property resolver that resolves property values against configuration
/// property sources.
///
/// This is the Rust equivalent of Next Boot's
/// `ConfigurationPropertySourcesPropertyResolver`.
///
/// Note: the resolution methods are exposed directly instead of implementing
/// [`PropertyResolver`](next_web_core::env::PropertyResolver). That trait's
/// `get_property_or_default` method returns a `&str` borrowed from `&self`,
/// which cannot be implemented for a caller-provided default value.
pub struct ConfigurationPropertySourcesPropertyResolver {
    sources: NextConfigurationPropertySources,
    placeholder_prefix: String,
    placeholder_suffix: String,
    ignore_unresolvable_nested_placeholders: bool,
}

impl ConfigurationPropertySourcesPropertyResolver {
    /// Creates a new resolver for the given configuration property sources.
    ///
    /// # Arguments
    ///
    /// * `sources` - The configuration property sources to resolve against.
    pub fn new(sources: NextConfigurationPropertySources) -> Self {
        Self {
            sources,
            placeholder_prefix: DEFAULT_PLACEHOLDER_PREFIX.to_owned(),
            placeholder_suffix: DEFAULT_PLACEHOLDER_SUFFIX.to_owned(),
            ignore_unresolvable_nested_placeholders: false,
        }
    }

    /// Returns whether the property with the given key can be resolved.
    ///
    /// # Arguments
    ///
    /// * `key` - The name of the property.
    pub fn contains_property(&self, key: &str) -> bool {
        self.lookup_property(key).is_some()
    }

    /// Resolves the property with the given key, or `None` when it cannot be
    /// resolved.
    ///
    /// Placeholders in the resolved value are resolved as well.
    ///
    /// # Arguments
    ///
    /// * `key` - The name of the property.
    pub fn get_property(&self, key: &str) -> Option<String> {
        let value = self.lookup_property(key)?;

        Some(self.resolve_placeholders_at_depth(&value, 0).0)
    }

    /// Resolves the property with the given key, falling back to the given
    /// default value when it cannot be resolved.
    ///
    /// # Arguments
    ///
    /// * `key` - The name of the property.
    /// * `default_value` - The value to use when the property is absent.
    pub fn get_property_or_default(&self, key: &str, default_value: &str) -> String {
        self.get_property(key)
            .unwrap_or_else(|| default_value.to_owned())
    }

    /// Resolves the property with the given key, which must be present.
    ///
    /// # Arguments
    ///
    /// * `key` - The name of the property.
    ///
    /// # Errors
    ///
    /// Returns [`IllegalError`] when the property cannot be resolved.
    pub fn get_required_property(&self, key: &str) -> Result<String, IllegalError> {
        self.get_property(key).ok_or_else(|| {
            IllegalError::IllegalStateError(format!("Could not resolve placeholder '{key}'"))
        })
    }

    /// Resolves the placeholders in the given text, leaving placeholders that
    /// cannot be resolved untouched.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to resolve.
    pub fn resolve_placeholders(&self, text: &str) -> String {
        self.resolve_placeholders_at_depth(text, 0).0
    }

    /// Resolves the placeholders in the given text, which must all be
    /// resolvable.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to resolve.
    ///
    /// # Errors
    ///
    /// Returns [`IllegalError`] when a placeholder cannot be resolved.
    pub fn resolve_required_placeholders(&self, text: &str) -> Result<String, IllegalError> {
        let (resolved, all_resolved) = self.resolve_placeholders_at_depth(text, 0);

        if all_resolved {
            Ok(resolved)
        } else {
            Err(IllegalError::IllegalStateError(format!(
                "Could not resolve placeholder in \"{text}\""
            )))
        }
    }

    /// Sets the prefix of placeholders, `${` by default.
    ///
    /// # Arguments
    ///
    /// * `placeholder_prefix` - The prefix to use.
    pub fn set_placeholder_prefix(&mut self, placeholder_prefix: impl Into<String>) {
        self.placeholder_prefix = placeholder_prefix.into();
    }

    /// Sets the suffix of placeholders, `}` by default.
    ///
    /// # Arguments
    ///
    /// * `placeholder_suffix` - The suffix to use.
    pub fn set_placeholder_suffix(&mut self, placeholder_suffix: impl Into<String>) {
        self.placeholder_suffix = placeholder_suffix.into();
    }

    /// Sets whether placeholders inside a placeholder that cannot be resolved
    /// are ignored.
    ///
    /// # Arguments
    ///
    /// * `ignore` - Whether unresolvable nested placeholders are ignored.
    pub fn set_ignore_unresolvable_nested_placeholders(&mut self, ignore: bool) {
        self.ignore_unresolvable_nested_placeholders = ignore;
    }

    /// Returns the value of the property with the given key, without resolving
    /// placeholders.
    ///
    /// # Arguments
    ///
    /// * `key` - The name of the property.
    fn lookup_property(&self, key: &str) -> Option<String> {
        let name = ConfigurationPropertyName::of(key).ok()?;

        self.sources
            .iter()
            .find_map(|source| source.get_configuration_property(&name))
            .map(|property| property.get_value().to_string())
    }

    /// Resolves the placeholders in the given text.
    ///
    /// Returns the resolved text together with whether every placeholder could
    /// be resolved.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to resolve.
    /// * `depth` - The current nesting depth, used to stop recursion.
    fn resolve_placeholders_at_depth(&self, text: &str, depth: usize) -> (String, bool) {
        let mut result = String::new();
        let mut all_resolved = true;
        let mut remaining = text;

        while let Some(start) = remaining.find(self.placeholder_prefix.as_str()) {
            result.push_str(&remaining[..start]);

            let after_prefix = &remaining[start + self.placeholder_prefix.len()..];
            let Some(end) = self.find_placeholder_end(after_prefix) else {
                // Without a suffix the remainder is not a placeholder.
                result.push_str(&remaining[start..]);
                return (result, all_resolved);
            };

            let placeholder = &after_prefix[..end];
            let placeholder_length = end + self.placeholder_suffix.len();
            let original =
                &remaining[start..start + self.placeholder_prefix.len() + placeholder_length];

            let (placeholder, nested_resolved) = if depth < MAX_PLACEHOLDER_DEPTH {
                self.resolve_placeholders_at_depth(placeholder, depth + 1)
            } else {
                (placeholder.to_owned(), true)
            };

            if !nested_resolved && !self.ignore_unresolvable_nested_placeholders {
                result.push_str(original);
                all_resolved = false;
            } else {
                let (key, default_value) = match placeholder.split_once(DEFAULT_VALUE_SEPARATOR) {
                    Some((key, default_value)) => (key, Some(default_value)),
                    None => (placeholder.as_str(), None),
                };

                match self.lookup_property(key) {
                    Some(value) => {
                        let (value, value_resolved) =
                            self.resolve_placeholders_at_depth(&value, depth + 1);
                        result.push_str(&value);
                        all_resolved &= value_resolved;
                    }
                    None => match default_value {
                        Some(default_value) => result.push_str(default_value),
                        None => {
                            result.push_str(original);
                            all_resolved = false;
                        }
                    },
                }
            }

            remaining = &after_prefix[placeholder_length..];
        }

        result.push_str(remaining);

        (result, all_resolved)
    }

    /// Returns the index of the suffix closing the placeholder that starts at
    /// the beginning of the given text, accounting for nested placeholders.
    ///
    /// # Arguments
    ///
    /// * `text` - The text following the placeholder prefix.
    fn find_placeholder_end(&self, text: &str) -> Option<usize> {
        let mut depth = 1;
        let mut index = 0;

        while index < text.len() {
            let remaining = &text[index..];

            if remaining.starts_with(self.placeholder_prefix.as_str()) {
                depth += 1;
                index += self.placeholder_prefix.len();
                continue;
            }

            if remaining.starts_with(self.placeholder_suffix.as_str()) {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
                index += self.placeholder_suffix.len();
                continue;
            }

            index += remaining.chars().next().map(char::len_utf8).unwrap_or(1);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        context::properties::source::NextConfigurationPropertySources, env::MapPropertySource,
    };
    use next_web_core::{env::MutablePropertySources, util::indexmap::IndexMap};

    /// Creates a resolver over the given properties.
    fn resolver(properties: &[(&str, &str)]) -> ConfigurationPropertySourcesPropertyResolver {
        let mut sources = MutablePropertySources::new();
        sources.add_last(Box::new(MapPropertySource::new(
            "test".to_owned(),
            properties
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect::<IndexMap<String, String>>(),
        )));

        ConfigurationPropertySourcesPropertyResolver::new(NextConfigurationPropertySources::new(
            &sources,
        ))
    }

    #[test]
    fn resolves_properties() {
        let resolver = resolver(&[("next.application.name", "demo")]);

        assert!(resolver.contains_property("next.application.name"));
        assert_eq!(
            resolver.get_property("next.application.name"),
            Some("demo".to_owned())
        );
        assert_eq!(resolver.get_property("next.application.version"), None);
        assert_eq!(
            resolver.get_property_or_default("next.application.version", "1.0.0"),
            "1.0.0"
        );
    }

    #[test]
    fn resolves_placeholders() {
        let resolver = resolver(&[
            ("app.name", "demo"),
            ("app.description", "the ${app.name} application"),
            ("app.unknown", "${app.missing}"),
        ]);

        assert_eq!(
            resolver.resolve_placeholders("the ${app.name} application"),
            "the demo application"
        );
        assert_eq!(
            resolver.resolve_placeholders("${app.description}"),
            "the demo application"
        );
        assert_eq!(
            resolver.resolve_placeholders("${app.name:fallback}"),
            "demo"
        );
        assert_eq!(
            resolver.resolve_placeholders("${app.missing:fallback}"),
            "fallback"
        );
        assert_eq!(
            resolver.resolve_placeholders("${app.missing}"),
            "${app.missing}"
        );
        // The property is present, but the placeholder in its value cannot be
        // resolved, so the unresolved text is kept.
        assert_eq!(
            resolver.resolve_placeholders("${app.unknown}"),
            "${app.missing}"
        );
        assert!(resolver
            .resolve_required_placeholders("${app.missing}")
            .is_err());
        assert_eq!(
            resolver.resolve_required_placeholders("${app.name}").ok(),
            Some("demo".to_owned())
        );
    }

    #[test]
    fn resolves_nested_placeholders() {
        let resolver = resolver(&[
            ("app.default", "fallback"),
            ("app.name", "demo"),
            ("app.value", "the ${app.missing:${app.default}} value"),
        ]);

        assert_eq!(
            resolver.resolve_placeholders("${app.value}"),
            "the fallback value"
        );
    }

    #[test]
    fn reports_required_properties_that_are_absent() {
        let resolver = resolver(&[("app.name", "demo")]);

        assert_eq!(
            resolver.get_required_property("app.name").ok(),
            Some("demo".to_owned())
        );
        assert!(resolver.get_required_property("app.missing").is_err());
    }
}
