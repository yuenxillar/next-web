//! Resolution of `#{NAME}` placeholders that read environment variables.
//!
//! The syntax is intentionally distinct from the `${name}` syntax of a
//! configuration property placeholder, so a value can refer to an environment
//! variable without taking part in the lookup of the configuration properties.
//!
//! A placeholder takes the form `#{NAME}` or `#{NAME:default}`. The default
//! value follows the first `:`, which allows a default such as a JDBC URL to
//! contain colons itself:
//!
//! ```text
//! #{DATABASE_URL:jdbc:mysql://localhost:3306/default}
//! ```

/// The prefix that opens an environment variable placeholder.
const PLACEHOLDER_PREFIX: &str = "#{";

/// The suffix that closes an environment variable placeholder.
const PLACEHOLDER_SUFFIX: char = '}';

/// The character that separates the name of an environment variable from its
/// default value.
const DEFAULT_VALUE_SEPARATOR: char = ':';

/// The maximum number of nested environment variable placeholders that are
/// resolved.
const MAX_PLACEHOLDER_DEPTH: usize = 8;

/// An environment variable placeholder whose variable is not set and which
/// declares no default value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UnresolvedEnvironmentVariable {
    name: String,
    placeholder: String,
}

impl UnresolvedEnvironmentVariable {
    /// Creates an unresolved environment variable.
    fn new(name: impl Into<String>, placeholder: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            placeholder: placeholder.into(),
        }
    }

    /// Returns the name of the environment variable.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// Returns the placeholder as it was written.
    pub(crate) fn placeholder(&self) -> &str {
        &self.placeholder
    }

    /// Returns why the placeholder could not be resolved.
    pub(crate) fn reason(&self) -> &'static str {
        "the variable is not set and no default value is declared"
    }

    /// Returns an example that declares a default value for the variable.
    pub(crate) fn default_example(&self) -> String {
        format!("#{{{}:default}}", self.name)
    }
}

/// Resolves `#{NAME}` and `#{NAME:default}` placeholders against the
/// environment variables of the process.
pub(crate) struct EnvironmentVariableResolver;

impl EnvironmentVariableResolver {
    /// Resolves the environment variable placeholders of the given text.
    ///
    /// A placeholder whose variable is not set and which declares no default
    /// value is kept as it was written and reported in the returned list.
    pub(crate) fn resolve(text: &str) -> (String, Vec<UnresolvedEnvironmentVariable>) {
        Self::resolve_with(text, &|name| std::env::var(name).ok())
    }

    /// Returns the environment variable placeholders of the given text that
    /// cannot be resolved and that declare no default value.
    ///
    /// This is used by the binder to report a missing environment variable for
    /// a value the environment has already resolved as far as it could.
    pub(crate) fn unresolved_in(text: &str) -> Vec<UnresolvedEnvironmentVariable> {
        Self::resolve_with(text, &|_| None).1
    }

    /// Resolves the environment variable placeholders of the given text with
    /// the supplied lookup.
    ///
    /// The lookup is injectable so that the resolution can be tested without
    /// changing the environment variables of the process.
    fn resolve_with<F>(text: &str, lookup: &F) -> (String, Vec<UnresolvedEnvironmentVariable>)
    where
        F: Fn(&str) -> Option<String>,
    {
        let mut unresolved = Vec::new();
        let value = Self::resolve_at_depth(text, lookup, 0, &mut unresolved);
        (value, unresolved)
    }

    /// Resolves the placeholders of the given text at the given nesting depth.
    fn resolve_at_depth<F>(
        text: &str,
        lookup: &F,
        depth: usize,
        unresolved: &mut Vec<UnresolvedEnvironmentVariable>,
    ) -> String
    where
        F: Fn(&str) -> Option<String>,
    {
        let mut result = String::with_capacity(text.len());
        let mut remaining = text;

        while let Some(start) = remaining.find(PLACEHOLDER_PREFIX) {
            result.push_str(&remaining[..start]);

            let after_prefix = &remaining[start + PLACEHOLDER_PREFIX.len()..];
            let Some(end) = Self::find_placeholder_end(after_prefix) else {
                // Without a closing suffix the rest of the text is not a
                // placeholder.
                result.push_str(&remaining[start..]);
                return result;
            };

            let body = &after_prefix[..end];
            let placeholder_length = PLACEHOLDER_PREFIX.len() + end + PLACEHOLDER_SUFFIX.len_utf8();
            let original = &remaining[start..start + placeholder_length];

            match Self::resolve_placeholder(body, lookup, depth, unresolved) {
                Some(value) => result.push_str(&value),
                None => result.push_str(original),
            }

            remaining = &after_prefix[end + PLACEHOLDER_SUFFIX.len_utf8()..];
        }

        result.push_str(remaining);
        result
    }

    /// Resolves a single environment variable placeholder.
    ///
    /// Returns `None` when the placeholder is malformed, or when its variable
    /// is not set and no default value is declared.
    fn resolve_placeholder<F>(
        body: &str,
        lookup: &F,
        depth: usize,
        unresolved: &mut Vec<UnresolvedEnvironmentVariable>,
    ) -> Option<String>
    where
        F: Fn(&str) -> Option<String>,
    {
        let (name, default_value) = match body.split_once(DEFAULT_VALUE_SEPARATOR) {
            Some((name, default_value)) => (name, Some(default_value)),
            None => (body, None),
        };
        let name = name.trim();

        if !Self::is_valid_name(name) {
            return None;
        }

        if let Some(value) = lookup(name) {
            return Some(if depth < MAX_PLACEHOLDER_DEPTH {
                Self::resolve_at_depth(&value, lookup, depth + 1, unresolved)
            } else {
                value
            });
        }

        if let Some(default_value) = default_value {
            return Some(if depth < MAX_PLACEHOLDER_DEPTH {
                Self::resolve_at_depth(default_value, lookup, depth + 1, unresolved)
            } else {
                default_value.to_owned()
            });
        }

        unresolved.push(UnresolvedEnvironmentVariable::new(
            name,
            format!("{PLACEHOLDER_PREFIX}{body}{PLACEHOLDER_SUFFIX}"),
        ));
        None
    }

    /// Returns the index of the suffix that closes the placeholder starting at
    /// the beginning of the given text, accounting for nested placeholders.
    fn find_placeholder_end(text: &str) -> Option<usize> {
        let mut depth = 1;
        let mut index = 0;

        while index < text.len() {
            let remaining = &text[index..];

            if remaining.starts_with(PLACEHOLDER_PREFIX) {
                depth += 1;
                index += PLACEHOLDER_PREFIX.len();
                continue;
            }

            if remaining.starts_with(PLACEHOLDER_SUFFIX) {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
                index += PLACEHOLDER_SUFFIX.len_utf8();
                continue;
            }

            index += remaining.chars().next().map(char::len_utf8).unwrap_or(1);
        }

        None
    }

    /// Returns whether the given name is an environment variable name.
    fn is_valid_name(name: &str) -> bool {
        let mut characters = name.chars();

        match characters.next() {
            Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
            _ => return false,
        }

        characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns a lookup over the given environment variables.
    fn lookup<'a>(environment: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            environment
                .iter()
                .find_map(|(key, value)| (*key == name).then(|| (*value).to_owned()))
        }
    }

    #[test]
    fn resolves_an_environment_variable() {
        let environment = lookup(&[("DATABASE_URL", "jdbc:mysql://db:3306/app")]);

        assert_eq!(
            EnvironmentVariableResolver::resolve_with("url is #{DATABASE_URL}", &environment),
            ("url is jdbc:mysql://db:3306/app".to_owned(), Vec::new())
        );
    }

    #[test]
    fn uses_the_default_value_when_the_variable_is_missing() {
        let environment = lookup(&[]);

        assert_eq!(
            EnvironmentVariableResolver::resolve_with(
                "#{DATABASE_URL:jdbc:mysql://localhost:3306/default}",
                &environment
            ),
            ("jdbc:mysql://localhost:3306/default".to_owned(), Vec::new())
        );
    }

    #[test]
    fn keeps_and_reports_a_placeholder_without_a_default_value() {
        let environment = lookup(&[]);

        let (value, unresolved) =
            EnvironmentVariableResolver::resolve_with("#{DATABASE_URL}", &environment);

        assert_eq!(value, "#{DATABASE_URL}");
        assert_eq!(unresolved.len(), 1);
        assert_eq!(unresolved[0].name(), "DATABASE_URL");
        assert_eq!(unresolved[0].placeholder(), "#{DATABASE_URL}");
    }

    #[test]
    fn resolves_nested_environment_variable_placeholders() {
        let environment = lookup(&[("MYSQL_HOST", "db"), ("DATABASE_URL", "mysql://host")]);

        assert_eq!(
            EnvironmentVariableResolver::resolve_with(
                "#{DATABASE_URL:#{MYSQL_HOST:localhost}}",
                &environment
            ),
            ("mysql://host".to_owned(), Vec::new())
        );

        let environment = lookup(&[]);
        assert_eq!(
            EnvironmentVariableResolver::resolve_with(
                "#{DATABASE_URL:#{MYSQL_HOST:localhost}}",
                &environment
            ),
            ("localhost".to_owned(), Vec::new())
        );
    }

    #[test]
    fn leaves_text_without_placeholders_and_unclosed_placeholders() {
        let environment = lookup(&[]);

        assert_eq!(
            EnvironmentVariableResolver::resolve_with("a plain value", &environment),
            ("a plain value".to_owned(), Vec::new())
        );
        assert_eq!(
            EnvironmentVariableResolver::resolve_with("an unclosed #{DATABASE_URL", &environment),
            ("an unclosed #{DATABASE_URL".to_owned(), Vec::new())
        );
    }
}
