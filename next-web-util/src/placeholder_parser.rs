//! Parser for strings that have placeholder values in them.
//!
//! In its simplest form, a placeholder takes the form `${name}`, where `name` is
//! the key that can be resolved using a [`PlaceholderResolver`], `${` the prefix, and
//! `}` the suffix.
//!
//! A placeholder can also have a default value if its key does not represent a known
//! property. The default value is separated from the key using a `separator`. For
//! instance `${name:John}` resolves to `John` if the placeholder resolver does not
//! provide a value for the `name` property.
//!
//! Placeholders can also have a more complex structure, and the resolution of a given
//! key can involve the resolution of nested placeholders. Default values can also have
//! placeholders.
//!
//! For situations where the syntax of a valid placeholder matches a string that must be
//! rendered as is, the placeholder can be escaped using an `escape` character. For
//! instance `\${name}` resolves as `${name}`.
//!
//! The prefix, suffix, separator, and escape characters are configurable. Only the
//! prefix and suffix are mandatory, and the support for default values or escaping is
//! conditional on providing non-null values for them.
//!
//! This parser resolves placeholders as lazily as possible.

use std::collections::HashSet;
use std::fmt;

use crate::PlaceholderResolver;
use crate::error::PlaceholderResolutionError;

/// A set of well-known simple prefixes keyed by their matching suffix.
fn well_known_simple_prefixes(suffix: &str) -> Option<&'static str> {
    match suffix {
        "}" => Some("{"),
        "]" => Some("["),
        ")" => Some("("),
        _ => None,
    }
}

/// Parser for strings that have placeholder values in them.
///
/// Corresponds to the Java `PlaceholderParser` class.
pub struct PlaceholderParser {
    prefix: String,
    suffix: String,
    simple_prefix: String,
    separator: Option<String>,
    ignore_unresolvable_placeholders: bool,
    escape: Option<char>,
}

impl PlaceholderParser {
    /// Creates an instance using the specified input for the parser.
    ///
    /// # Arguments
    ///
    /// * `prefix` - the prefix that denotes the start of a placeholder
    /// * `suffix` - the suffix that denotes the end of a placeholder
    /// * `separator` - the separating character between the placeholder variable and
    ///   the associated default value, if any
    /// * `escape` - the character to use at the beginning of a placeholder prefix or
    ///   separator to escape it and render it as is
    /// * `ignore_unresolvable_placeholders` - whether unresolvable placeholders should be
    ///   ignored (`true`) or cause an error (`false`)
    pub fn new(
        prefix: String,
        suffix: String,
        separator: Option<String>,
        escape: Option<char>,
        ignore_unresolvable_placeholders: bool,
    ) -> Self {
        let simple_prefix = match well_known_simple_prefixes(&suffix) {
            Some(sp) if prefix.ends_with(sp) => sp.to_owned(),
            _ => prefix.clone(),
        };
        Self {
            prefix,
            suffix,
            simple_prefix,
            separator,
            ignore_unresolvable_placeholders,
            escape,
        }
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
        let parts = match self.parse(value, false) {
            Some(parts) => parts,
            None => return Ok(value.to_owned()),
        };
        let parsed_value = ParsedValue::new(value.to_owned(), parts);
        let mut resolution_context = PartResolutionContext::new(
            placeholder_resolver,
            self.prefix.clone(),
            self.suffix.clone(),
            self.ignore_unresolvable_placeholders,
            self,
        );
        parsed_value.resolve(&mut resolution_context)
    }

    /// Parses the given value into a list of [`Part`]s.
    ///
    /// Returns `None` if the value contains no placeholder at all.
    fn parse(&self, value: &str, in_placeholder: bool) -> Option<Vec<Part>> {
        let mut start_index = self.next_start_prefix(value, 0)?;

        let mut parts: Vec<Part> = Vec::with_capacity(4);
        let mut position = 0usize;

        loop {
            let end_index = self.next_valid_end_prefix(value, start_index);

            match end_index {
                None => {
                    // Not a valid placeholder, consume the prefix and continue.
                    add_text(value, position, start_index + self.prefix.len(), &mut parts);
                    position = start_index + self.prefix.len();
                    match self.next_start_prefix(value, position) {
                        Some(next) => start_index = next,
                        None => break,
                    }
                }
                Some(_) if self.is_escaped(value, start_index) => {
                    // Escaped placeholder: keep the text literal and skip the escape char.
                    add_text(value, position, start_index - self.escape_len(), &mut parts);
                    add_text(
                        value,
                        start_index,
                        start_index + self.prefix.len(),
                        &mut parts,
                    );
                    position = start_index + self.prefix.len();
                    match self.next_start_prefix(value, position) {
                        Some(next) => start_index = next,
                        None => break,
                    }
                }
                Some(end) => {
                    // Found a valid placeholder, recurse into its content.
                    add_text(value, position, start_index, &mut parts);
                    let placeholder = &value[start_index + self.prefix.len()..end];
                    match self.parse(placeholder, true) {
                        Some(placeholder_parts) => parts.extend(placeholder_parts),
                        None => parts.push(self.create_simple_placeholder_part(placeholder)),
                    }
                    position = end + self.suffix.len();
                    match self.next_start_prefix(value, position) {
                        Some(next) => start_index = next,
                        None => break,
                    }
                }
            }
        }

        // Add the remaining text.
        add_text(value, position, value.len(), &mut parts);

        if in_placeholder {
            let nested = self.create_nested_placeholder_part(value.to_owned(), parts);
            Some(vec![Part::Nested(nested)])
        } else {
            Some(parts)
        }
    }

    fn create_simple_placeholder_part(&self, text: &str) -> Part {
        let section = self.parse_section(text);
        Part::Simple(SimplePlaceholderPart::new(
            text.to_owned(),
            section.key,
            section.fallback,
        ))
    }

    fn create_nested_placeholder_part(
        &self,
        text: String,
        parts: Vec<Part>,
    ) -> NestedPlaceholderPart {
        if self.separator.is_none() {
            return NestedPlaceholderPart::new(text, parts, None);
        }
        let mut key_parts: Vec<Part> = Vec::with_capacity(parts.len());
        let mut default_parts: Vec<Part> = Vec::new();

        let mut iter = parts.into_iter().enumerate();
        while let Some((_i, part)) = iter.next() {
            match part {
                Part::Text(text_part) => {
                    let candidate = &text_part.text;
                    let section = self.parse_section(candidate);
                    key_parts.push(Part::Text(TextPart::new(section.key)));
                    if let Some(fallback) = section.fallback {
                        default_parts.push(Part::Text(TextPart::new(fallback)));
                        // Collect the remaining parts into default_parts.
                        for (_, remaining) in iter {
                            default_parts.push(remaining);
                        }
                        return NestedPlaceholderPart::new(text, key_parts, Some(default_parts));
                    }
                }
                other => key_parts.push(other),
            }
        }
        NestedPlaceholderPart::new(text, key_parts, None)
    }

    /// Parses an input value that may contain a separator character.
    ///
    /// If a valid separator character has been identified, the given `value` is split
    /// between a `key` and a `fallback`. If not, only the `key` is set.
    ///
    /// The returned key may be different from the original value as escaped separators,
    /// if any, are resolved.
    fn parse_section(&self, value: &str) -> ParsedSection {
        let separator = match &self.separator {
            Some(sep) => sep,
            None => return ParsedSection::new(value.to_owned(), None),
        };
        if !value.contains(separator.as_str()) {
            return ParsedSection::new(value.to_owned(), None);
        }
        let mut position = 0usize;
        let mut index = value[position..]
            .find(separator.as_str())
            .map(|i| i + position);
        let mut buffer = String::with_capacity(value.len());

        while let Some(idx) = index {
            if self.is_escaped(value, idx) {
                // Accumulate, without the escape character.
                buffer.push_str(&value[position..idx - self.escape_len()]);
                buffer.push_str(&value[idx..idx + separator.len()]);
                position = idx + separator.len();
                index = value[position..]
                    .find(separator.as_str())
                    .map(|i| i + position);
            } else {
                buffer.push_str(&value[position..idx]);
                let key = buffer;
                let fallback = value[idx + separator.len()..].to_owned();
                return ParsedSection::new(key, Some(fallback));
            }
        }
        buffer.push_str(&value[position..]);
        ParsedSection::new(buffer, None)
    }

    #[inline]
    fn next_start_prefix(&self, value: &str, index: usize) -> Option<usize> {
        value[index..].find(self.prefix.as_str()).map(|i| i + index)
    }

    /// Finds the next valid closing suffix starting at `start_index`.
    ///
    /// Returns `None` if no valid end is found.
    fn next_valid_end_prefix(&self, value: &str, start_index: usize) -> Option<usize> {
        let mut index = start_index + self.prefix.len();
        let mut within_nested_placeholder = 0usize;

        while index < value.len() {
            if value[index..].starts_with(self.suffix.as_str()) {
                if within_nested_placeholder > 0 {
                    within_nested_placeholder -= 1;
                    index += self.suffix.len();
                } else {
                    return Some(index);
                }
            } else if value[index..].starts_with(self.simple_prefix.as_str()) {
                within_nested_placeholder += 1;
                index += self.simple_prefix.len();
            } else {
                // Advance a full UTF-8 character to stay on a char boundary.
                index += value[index..]
                    .chars()
                    .next()
                    .map(char::len_utf8)
                    .unwrap_or(1);
            }
        }
        None
    }

    #[inline]
    fn is_escaped(&self, value: &str, index: usize) -> bool {
        match self.escape {
            Some(escape) => index > 0 && value[..index].ends_with(escape),
            None => false,
        }
    }

    #[inline]
    fn escape_len(&self) -> usize {
        self.escape.map(char::len_utf8).unwrap_or(0)
    }
}

/// Appends the text slice `value[start..end]` to `parts`, merging with the previous
/// [`TextPart`] if possible to reduce the number of allocated parts.
#[inline]
fn add_text(value: &str, start: usize, end: usize, parts: &mut Vec<Part>) {
    if start >= end {
        return;
    }
    let text = &value[start..end];
    if let Some(Part::Text(last)) = parts.last_mut() {
        last.text.push_str(text);
    } else {
        parts.push(Part::Text(TextPart::new(text.to_owned())));
    }
}

/// A representation of the parsing of an input string.
struct ParsedValue {
    text: String,
    parts: Vec<Part>,
}

impl ParsedValue {
    fn new(text: String, parts: Vec<Part>) -> Self {
        Self { text, parts }
    }

    fn resolve(
        &self,
        resolution_context: &mut PartResolutionContext<'_>,
    ) -> Result<String, PlaceholderResolutionError> {
        resolve_all(&self.parts, resolution_context)
            .map_err(|err| err.with_value(self.text.clone()))
    }
}

/// A parsed section of a placeholder, split into a key and an optional fallback.
struct ParsedSection {
    key: String,
    fallback: Option<String>,
}

impl ParsedSection {
    fn new(key: String, fallback: Option<String>) -> Self {
        Self { key, fallback }
    }
}

/// Provides the necessary context to handle and resolve underlying placeholders.
struct PartResolutionContext<'a> {
    prefix: String,
    suffix: String,
    ignore_unresolvable_placeholders: bool,
    parser: &'a PlaceholderParser,
    resolver: &'a dyn PlaceholderResolver,
    visited_placeholders: Option<HashSet<String>>,
}

impl<'a> PartResolutionContext<'a> {
    fn new(
        resolver: &'a dyn PlaceholderResolver,
        prefix: String,
        suffix: String,
        ignore_unresolvable_placeholders: bool,
        parser: &'a PlaceholderParser,
    ) -> Self {
        Self {
            prefix,
            suffix,
            ignore_unresolvable_placeholders,
            parser,
            resolver,
            visited_placeholders: None,
        }
    }

    fn resolve_placeholder(&self, placeholder_name: &str) -> Option<String> {
        let value = self.resolver.resolve_placeholder(placeholder_name);
        if value.is_some() && tracing::enabled!(tracing::Level::TRACE) {
            tracing::trace!(placeholder = placeholder_name, "Resolved placeholder");
        }
        value
    }

    fn handle_unresolvable_placeholder(
        &self,
        key: &str,
        text: &str,
    ) -> Result<String, PlaceholderResolutionError> {
        if self.ignore_unresolvable_placeholders {
            return Ok(self.to_placeholder_text(key));
        }
        let original_value = if key != text {
            Some(self.to_placeholder_text(text))
        } else {
            None
        };
        Err(PlaceholderResolutionError::new(
            format!("Could not resolve placeholder '{key}'"),
            key,
            original_value,
        ))
    }

    #[inline]
    fn to_placeholder_text(&self, text: &str) -> String {
        let mut result = String::with_capacity(self.prefix.len() + text.len() + self.suffix.len());
        result.push_str(&self.prefix);
        result.push_str(text);
        result.push_str(&self.suffix);
        result
    }

    fn parse(&self, text: &str) -> Option<Vec<Part>> {
        self.parser.parse(text, false)
    }

    fn flag_placeholder_as_visited(
        &mut self,
        placeholder: &str,
    ) -> Result<(), PlaceholderResolutionError> {
        let visited = self
            .visited_placeholders
            .get_or_insert_with(|| HashSet::with_capacity(4));
        if !visited.insert(placeholder.to_owned()) {
            return Err(PlaceholderResolutionError::new(
                format!("Circular placeholder reference '{placeholder}'"),
                placeholder,
                None,
            ));
        }
        Ok(())
    }

    fn remove_placeholder(&mut self, placeholder: &str) {
        if let Some(visited) = &mut self.visited_placeholders {
            visited.remove(placeholder);
        }
    }
}

/// A part is a section of a string containing placeholders to replace.
enum Part {
    Text(TextPart),
    Simple(SimplePlaceholderPart),
    Nested(NestedPlaceholderPart),
}

impl Part {
    fn resolve(
        &self,
        resolution_context: &mut PartResolutionContext<'_>,
    ) -> Result<String, PlaceholderResolutionError> {
        match self {
            Part::Text(p) => Ok(p.text.clone()),
            Part::Simple(p) => p.resolve(resolution_context),
            Part::Nested(p) => p.resolve(resolution_context),
        }
    }

    fn text(&self) -> &str {
        match self {
            Part::Text(p) => &p.text,
            Part::Simple(p) => &p.base.text,
            Part::Nested(p) => &p.base.text,
        }
    }
}

/// Resolves all parts and concatenates the results.
fn resolve_all(
    parts: &[Part],
    resolution_context: &mut PartResolutionContext<'_>,
) -> Result<String, PlaceholderResolutionError> {
    let mut sb = String::with_capacity(parts.iter().map(|p| p.text().len()).sum());
    for part in parts {
        sb.push_str(&part.resolve(resolution_context)?);
    }
    Ok(sb)
}

/// A base [`Part`] implementation.
struct AbstractPart {
    text: String,
}

impl AbstractPart {
    fn new(text: String) -> Self {
        Self { text }
    }

    /// Resolves the placeholder with the given `key` recursively.
    ///
    /// If the result of such resolution returns other placeholders, those are resolved
    /// as well until the resolution no longer contains any placeholders.
    fn resolve_recursively(
        &self,
        resolution_context: &mut PartResolutionContext<'_>,
        key: &str,
    ) -> Result<Option<String>, PlaceholderResolutionError> {
        let resolved_value = match resolution_context.resolve_placeholder(key) {
            Some(v) => v,
            None => return Ok(None),
        };
        // Check if we need to recursively resolve that value.
        let nested_parts = match resolution_context.parse(&resolved_value) {
            Some(p) => p,
            None => return Ok(Some(resolved_value)),
        };
        resolution_context.flag_placeholder_as_visited(key)?;
        let value = ParsedValue::new(resolved_value, nested_parts).resolve(resolution_context);
        resolution_context.remove_placeholder(key);
        value.map(Some)
    }
}

/// A [`Part`] implementation that does not contain a valid placeholder.
struct TextPart {
    text: String,
}

impl TextPart {
    fn new(text: String) -> Self {
        Self { text }
    }
}

/// A [`Part`] implementation that represents a single placeholder with a hard-coded
/// fallback.
struct SimplePlaceholderPart {
    base: AbstractPart,
    key: String,
    fallback: Option<String>,
}

impl SimplePlaceholderPart {
    fn new(text: String, key: String, fallback: Option<String>) -> Self {
        Self {
            base: AbstractPart::new(text),
            key,
            fallback,
        }
    }

    fn resolve(
        &self,
        resolution_context: &mut PartResolutionContext<'_>,
    ) -> Result<String, PlaceholderResolutionError> {
        if let Some(value) = self.resolve_recursively(resolution_context)? {
            Ok(value)
        } else if let Some(fallback) = &self.fallback {
            Ok(fallback.clone())
        } else {
            resolution_context.handle_unresolvable_placeholder(&self.key, &self.base.text)
        }
    }

    fn resolve_recursively(
        &self,
        resolution_context: &mut PartResolutionContext<'_>,
    ) -> Result<Option<String>, PlaceholderResolutionError> {
        if self.base.text != self.key
            && let Some(value) = self
                .base
                .resolve_recursively(resolution_context, &self.base.text)?
        {
            return Ok(Some(value));
        }
        self.base.resolve_recursively(resolution_context, &self.key)
    }
}

/// A [`Part`] implementation that represents a single placeholder containing nested
/// placeholders.
struct NestedPlaceholderPart {
    base: AbstractPart,
    key_parts: Vec<Part>,
    default_parts: Option<Vec<Part>>,
}

impl NestedPlaceholderPart {
    fn new(text: String, key_parts: Vec<Part>, default_parts: Option<Vec<Part>>) -> Self {
        Self {
            base: AbstractPart::new(text),
            key_parts,
            default_parts,
        }
    }

    fn resolve(
        &self,
        resolution_context: &mut PartResolutionContext<'_>,
    ) -> Result<String, PlaceholderResolutionError> {
        let resolved_key = resolve_all(&self.key_parts, resolution_context)?;
        if let Some(value) = self
            .base
            .resolve_recursively(resolution_context, &resolved_key)?
        {
            Ok(value)
        } else if let Some(default_parts) = &self.default_parts {
            resolve_all(default_parts, resolution_context)
        } else {
            resolution_context.handle_unresolvable_placeholder(&resolved_key, &self.base.text)
        }
    }
}

impl fmt::Display for ParsedSection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ParsedSection(key={}, fallback={:?})",
            self.key, self.fallback
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MapResolver {
        map: std::collections::HashMap<String, String>,
    }

    impl MapResolver {
        fn new(pairs: &[(&str, &str)]) -> Self {
            Self {
                map: pairs
                    .iter()
                    .map(|(k, v)| (String::from(*k), String::from(*v)))
                    .collect(),
            }
        }
    }

    impl PlaceholderResolver for MapResolver {
        fn resolve_placeholder(&self, name: &str) -> Option<String> {
            self.map.get(name).cloned()
        }
    }

    fn parser() -> PlaceholderParser {
        PlaceholderParser::new(
            "${".to_owned(),
            "}".to_owned(),
            Some(":".to_owned()),
            Some('\\'),
            true,
        )
    }

    fn strict_parser() -> PlaceholderParser {
        PlaceholderParser::new(
            "${".to_owned(),
            "}".to_owned(),
            Some(":".to_owned()),
            Some('\\'),
            false,
        )
    }

    #[test]
    fn resolves_simple_placeholder() {
        let resolver = MapResolver::new(&[("name", "World")]);
        let result = parser()
            .replace_placeholders("Hello ${name}", &resolver)
            .unwrap();
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn uses_fallback_when_key_missing() {
        let resolver = MapResolver::new(&[]);
        let result = parser()
            .replace_placeholders("Hello ${name:John}", &resolver)
            .unwrap();
        assert_eq!(result, "Hello John");
    }

    #[test]
    fn escapes_placeholder() {
        let resolver = MapResolver::new(&[("name", "World")]);
        let result = parser()
            .replace_placeholders(r"\${name}", &resolver)
            .unwrap();
        assert_eq!(result, "${name}");
    }

    #[test]
    fn resolves_nested_placeholder_in_key() {
        let resolver = MapResolver::new(&[("a", "b"), ("b", "value")]);
        let result = parser().replace_placeholders("${${a}}", &resolver).unwrap();
        assert_eq!(result, "value");
    }

    #[test]
    fn resolves_placeholder_within_resolved_value() {
        let resolver = MapResolver::new(&[("a", "${b}"), ("b", "value")]);
        let result = parser().replace_placeholders("${a}", &resolver).unwrap();
        assert_eq!(result, "value");
    }

    #[test]
    fn resolves_placeholder_in_fallback() {
        let resolver = MapResolver::new(&[("b", "value")]);
        let result = parser()
            .replace_placeholders("${a:${b}}", &resolver)
            .unwrap();
        assert_eq!(result, "value");
    }

    #[test]
    fn ignores_unresolvable_placeholder_when_configured() {
        let resolver = MapResolver::new(&[]);
        let result = parser()
            .replace_placeholders("Hello ${missing}", &resolver)
            .unwrap();
        assert_eq!(result, "Hello ${missing}");
    }

    #[test]
    fn errors_on_unresolvable_placeholder_when_not_ignored() {
        let resolver = MapResolver::new(&[]);
        let err = strict_parser()
            .replace_placeholders("Hello ${missing}", &resolver)
            .unwrap_err();
        assert!(err.message().contains("Could not resolve placeholder"));
        assert_eq!(err.placeholder(), "missing");
    }

    #[test]
    fn errors_on_circular_placeholder_reference() {
        let resolver = MapResolver::new(&[("a", "${a}")]);
        let err = strict_parser()
            .replace_placeholders("${a}", &resolver)
            .unwrap_err();
        assert!(err.message().contains("Circular placeholder reference"));
        assert_eq!(err.placeholder(), "a");
    }

    #[test]
    fn attaches_original_value_to_error() {
        let resolver = MapResolver::new(&[("a", "missing")]);
        let err = strict_parser()
            .replace_placeholders("${${a}}", &resolver)
            .unwrap_err();
        assert_eq!(err.placeholder(), "missing");
        assert_eq!(err.original_value(), Some("${${a}}"));
    }
}
