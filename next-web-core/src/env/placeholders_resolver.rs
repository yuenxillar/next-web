//! Resolution of the placeholders of property values and of text.

/// The prefix that opens a placeholder.
const DEFAULT_PLACEHOLDER_PREFIX: &str = "${";

/// The suffix that closes a placeholder.
const DEFAULT_PLACEHOLDER_SUFFIX: &str = "}";

/// The character that separates the key of a placeholder from its default
/// value.
const DEFAULT_VALUE_SEPARATOR: char = ':';

/// The maximum number of nested placeholders that are resolved.
const MAX_PLACEHOLDER_DEPTH: usize = 8;

/// Resolves the placeholders of property values and of arbitrary text.
///
/// A placeholder refers to a property by its key and may declare a default
/// value, for example `${next.application.name}` or `${next.server.port:8080}`.
/// Placeholders may be nested, in which case the default value of a placeholder
/// is a placeholder itself.
pub(crate) struct PlaceholdersResolver {
    placeholder_prefix: String,
    placeholder_suffix: String,
}

impl PlaceholdersResolver {
    /// Creates a resolver that uses `${` and `}` as its placeholder markers.
    pub(crate) fn new() -> Self {
        Self {
            placeholder_prefix: DEFAULT_PLACEHOLDER_PREFIX.to_owned(),
            placeholder_suffix: DEFAULT_PLACEHOLDER_SUFFIX.to_owned(),
        }
    }

    /// Resolves the placeholders of the given text.
    ///
    /// The value of a placeholder is looked up with the given lookup. A
    /// placeholder that cannot be resolved is kept as it is written, unless it
    /// declares a default value.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to resolve the placeholders of.
    /// * `lookup` - The lookup used to resolve the key of a placeholder.
    ///
    /// # Returns
    ///
    /// The resolved text, together with whether every placeholder of the text
    /// could be resolved.
    pub(crate) fn resolve(
        &self,
        text: &str,
        lookup: &dyn Fn(&str) -> Option<String>,
    ) -> (String, bool) {
        self.resolve_at_depth(text, lookup, 0)
    }

    /// Resolves the placeholders of the given text.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to resolve the placeholders of.
    /// * `lookup` - The lookup used to resolve the key of a placeholder.
    /// * `depth` - The current nesting depth, used to stop the recursion.
    fn resolve_at_depth(
        &self,
        text: &str,
        lookup: &dyn Fn(&str) -> Option<String>,
        depth: usize,
    ) -> (String, bool) {
        let mut result = String::new();
        let mut all_resolved = true;
        let mut remaining = text;

        while let Some(start) = remaining.find(self.placeholder_prefix.as_str()) {
            result.push_str(&remaining[..start]);

            let after_prefix = &remaining[start + self.placeholder_prefix.len()..];
            let Some(end) = self.find_placeholder_end(after_prefix) else {
                // Without a suffix the rest of the text is not a placeholder.
                result.push_str(&remaining[start..]);
                return (result, all_resolved);
            };

            let placeholder = &after_prefix[..end];
            let placeholder_length = end + self.placeholder_suffix.len();
            let original =
                &remaining[start..start + self.placeholder_prefix.len() + placeholder_length];

            let (placeholder, nested_resolved) = if depth < MAX_PLACEHOLDER_DEPTH {
                self.resolve_at_depth(placeholder, lookup, depth + 1)
            } else {
                (placeholder.to_owned(), true)
            };

            if !nested_resolved {
                result.push_str(original);
                all_resolved = false;
            } else {
                match self.resolve_placeholder(&placeholder, lookup, depth) {
                    Some((value, value_resolved)) => {
                        result.push_str(&value);
                        all_resolved &= value_resolved;
                    }
                    None => {
                        result.push_str(original);
                        all_resolved = false;
                    }
                }
            }

            remaining = &after_prefix[placeholder_length..];
        }

        result.push_str(remaining);

        (result, all_resolved)
    }

    /// Resolves a single placeholder, or returns `None` when its key cannot be
    /// resolved and it declares no default value.
    ///
    /// # Arguments
    ///
    /// * `placeholder` - The text between the placeholder markers.
    /// * `lookup` - The lookup used to resolve the key of the placeholder.
    /// * `depth` - The current nesting depth, used to stop the recursion.
    fn resolve_placeholder(
        &self,
        placeholder: &str,
        lookup: &dyn Fn(&str) -> Option<String>,
        depth: usize,
    ) -> Option<(String, bool)> {
        let (key, default_value) = match placeholder.split_once(DEFAULT_VALUE_SEPARATOR) {
            Some((key, default_value)) => (key, Some(default_value)),
            None => (placeholder, None),
        };

        match lookup(key) {
            Some(value) => Some(self.resolve_at_depth(&value, lookup, depth + 1)),
            None => default_value.map(|default_value| (default_value.to_owned(), true)),
        }
    }

    /// Returns the index of the suffix that closes the placeholder starting at
    /// the beginning of the given text, accounting for nested placeholders.
    ///
    /// # Arguments
    ///
    /// * `text` - The text that follows the placeholder prefix.
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

impl Default for PlaceholdersResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// Returns a lookup over the given properties.
    fn lookup(properties: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let properties: HashMap<String, String> = properties
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        move |key: &str| properties.get(key).cloned()
    }

    #[test]
    fn resolves_a_placeholder_from_the_lookup() {
        let resolver = PlaceholdersResolver::new();
        let lookup = lookup(&[("app.name", "demo")]);

        assert_eq!(
            resolver.resolve("the ${app.name} application", &lookup),
            ("the demo application".to_owned(), true)
        );
    }

    #[test]
    fn keeps_a_placeholder_that_cannot_be_resolved() {
        let resolver = PlaceholdersResolver::new();
        let lookup = lookup(&[]);

        assert_eq!(
            resolver.resolve("the ${app.name} application", &lookup),
            ("the ${app.name} application".to_owned(), false)
        );
    }

    #[test]
    fn resolves_the_default_value_of_a_placeholder() {
        let resolver = PlaceholdersResolver::new();
        let lookup = lookup(&[("app.name", "demo")]);

        assert_eq!(
            resolver.resolve("${app.name:fallback}", &lookup),
            ("demo".to_owned(), true)
        );
        assert_eq!(
            resolver.resolve("${app.missing:fallback}", &lookup),
            ("fallback".to_owned(), true)
        );
    }

    #[test]
    fn resolves_nested_placeholders() {
        let resolver = PlaceholdersResolver::new();
        let lookup = lookup(&[("app.default", "fallback"), ("app.name", "demo")]);

        assert_eq!(
            resolver.resolve("${app.missing:${app.default}}", &lookup),
            ("fallback".to_owned(), true)
        );
        assert_eq!(
            resolver.resolve("${app.name}", &lookup),
            ("demo".to_owned(), true)
        );
    }

    #[test]
    fn resolves_the_placeholders_of_the_value_of_a_placeholder() {
        let resolver = PlaceholdersResolver::new();
        let lookup = lookup(&[
            ("app.name", "demo"),
            ("app.description", "the ${app.name} app"),
        ]);

        assert_eq!(
            resolver.resolve("${app.description}", &lookup),
            ("the demo app".to_owned(), true)
        );
    }

    #[test]
    fn reports_the_text_of_a_placeholder_that_cannot_be_resolved() {
        let resolver = PlaceholdersResolver::new();
        let lookup = lookup(&[("app.unknown", "${app.missing}")]);

        assert_eq!(
            resolver.resolve("${app.unknown}", &lookup),
            ("${app.missing}".to_owned(), false)
        );
    }

    #[test]
    fn keeps_text_without_placeholders_and_unclosed_placeholders() {
        let resolver = PlaceholdersResolver::new();
        let lookup = lookup(&[]);

        assert_eq!(
            resolver.resolve("a plain value", &lookup),
            ("a plain value".to_owned(), true)
        );
        assert_eq!(
            resolver.resolve("an unclosed ${placeholder", &lookup),
            ("an unclosed ${placeholder".to_owned(), true)
        );
    }
}
