//! Banner implementation that prints from a source text [`Resource`].

use std::collections::HashMap;
use std::io::{self, Write};

use next_web_core::env::{Environment, PropertySource};
use next_web_core::io::Resource;
use tracing::warn;

use crate::ansi::{AnsiPropertySource, ANSI_PROPERTY_SOURCE_NAME};
use crate::{Banner, NextWebVersion};

/// Property holding the title of the application.
const APPLICATION_TITLE_PROPERTY: &str = "application.title";

/// Environment property holding the version of the application.
const APPLICATION_VERSION_SOURCE_PROPERTY: &str = "next.application.version";

/// Property holding the version of the application.
const APPLICATION_VERSION_PROPERTY: &str = "application.version";

/// Property holding the formatted version of the application.
const APPLICATION_FORMATTED_VERSION_PROPERTY: &str = "application.formatted-version";

/// Property holding the version of the framework.
const NEXT_VERSION_PROPERTY: &str = "next.version";

/// Property holding the formatted version of the framework.
const NEXT_FORMATTED_VERSION_PROPERTY: &str = "next.formatted-version";

/// Start of a property placeholder.
const PLACEHOLDER_PREFIX: &str = "${";

/// Separator between the key of a placeholder and its default value.
const VALUE_SEPARATOR: char = ':';

/// Maximum nesting depth accepted while resolving placeholders.
const MAX_PLACEHOLDER_DEPTH: usize = 32;

/// Banner implementation that prints from a source text [`Resource`].
///
/// The banner text is read from the resource and its `${...}` placeholders are
/// resolved against the environment properties and the properties the banner
/// provides itself, such as `${application.title}`, `${next.version}` and the
/// ANSI escape codes of [`crate::ansi`]. Placeholders referring to an unknown
/// property are kept verbatim, while known properties without a value are
/// replaced by an empty string.
///
/// A banner that cannot be read or printed is logged and discarded, so that a
/// broken banner never prevents the application from starting.
#[derive(Clone)]
pub struct ResourceBanner<'a> {
    /// The resource holding the banner text.
    resource: &'a dyn Resource,
    /// The title resolved by the `${application.title}` placeholder.
    application_title: Option<String>,
}

impl<'a> ResourceBanner<'a> {
    /// Creates a banner printing the text of the given resource.
    ///
    /// # Arguments
    ///
    /// * `resource` - The resource holding the banner text.
    ///
    /// # Returns
    ///
    /// A new [`ResourceBanner`].
    pub fn new(resource: &'a dyn Resource) -> Self {
        Self {
            resource,
            application_title: None,
        }
    }

    /// Sets the title resolved by the `${application.title}` placeholder.
    ///
    /// A binary has no package title to fall back on, so the title is supplied
    /// by the caller. When it is not set, the placeholder resolves to an empty
    /// string.
    ///
    /// # Arguments
    ///
    /// * `title` - The title of the application.
    ///
    /// # Returns
    ///
    /// This banner.
    pub fn with_application_title(mut self, title: impl Into<String>) -> Self {
        self.application_title = Some(title.into());
        self
    }

    /// Returns the underlying resource.
    pub fn resource(&self) -> &dyn Resource {
        self.resource
    }

    /// Returns the title resolved by the `${application.title}` placeholder.
    pub fn application_title(&self) -> Option<&str> {
        self.application_title.as_deref()
    }

    /// Reads the banner text and prints it to the given writer.
    ///
    /// The banner is read as UTF-8, invalid byte sequences being replaced
    /// rather than rejected.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment resolving the placeholders.
    /// * `out` - The writer receiving the banner.
    ///
    /// # Errors
    ///
    /// Returns an error when the resource cannot be read.
    fn print(&self, environment: &dyn Environment, out: &mut dyn Write) -> io::Result<()> {
        let banner = self.resource.get_content_as_string()?;
        let banner = self.resolve_placeholders(environment, &banner);

        // The banner is printed as a line of its own, keeping its own trailing
        // newline when it has one.
        writeln!(out, "{banner}")
    }

    /// Resolves the placeholders of the banner text.
    ///
    /// The text is resolved twice, mirroring the way the environment and the
    /// properties of the banner are layered:
    ///
    /// 1. The environment properties take precedence, followed by the title,
    ///    the ANSI escape codes and the version properties. Properties without
    ///    a value are left unresolved.
    /// 2. The title and the version properties are applied again with an empty
    ///    default, so that a placeholder such as `${application.title}` is
    ///    always replaced, even when no value is known.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment used to look up properties.
    /// * `text` - The banner text.
    ///
    /// # Returns
    ///
    /// The banner text with all resolvable placeholders replaced.
    fn resolve_placeholders(&self, environment: &dyn Environment, text: &str) -> String {
        let ansi = AnsiPropertySource::new(ANSI_PROPERTY_SOURCE_NAME, true);
        let application_version = environment.get_property(APPLICATION_VERSION_SOURCE_PROPERTY);
        let title = self.application_title.as_deref();
        let application_version = application_version.as_deref();

        let defaults = BannerProperties::new(title, application_version, None);
        let empty_defaults = BannerProperties::new(title, application_version, Some(""));

        let resolved = resolve_placeholders_with(text, &|name| {
            environment
                .get_property(name)
                .or_else(|| defaults.get(name))
                .or_else(|| ansi.property(name))
        });

        resolve_placeholders_with(&resolved, &|name| empty_defaults.get(name))
    }

    /// Returns a description of the resource, used for logging.
    fn description(&self) -> String {
        match self.resource.path() {
            Ok(path) => path.display().to_string(),
            Err(_) => self.resource.filename().unwrap_or("<unknown>").to_string(),
        }
    }
}

impl<'a> Banner for ResourceBanner<'a> {
    fn print_banner(&self, environment: &dyn Environment, out: &mut dyn Write) -> io::Result<()> {
        if let Err(error) = self.print(environment, out) {
            warn!(%error, resource = %self.description(), "banner not printable");
        }
        Ok(())
    }
}

/// The properties the banner adds below the environment properties.
struct BannerProperties {
    /// The title of the application, when known.
    application_title: Option<String>,
    /// The version properties, keyed by the placeholder resolving them.
    versions: HashMap<&'static str, Option<String>>,
    /// The value used for a property the banner cannot determine.
    ///
    /// The first resolution pass uses `None`, so that a placeholder without a
    /// value survives to the second pass; the second pass uses an empty string,
    /// so that the placeholder is always replaced.
    default_value: Option<String>,
}

impl BannerProperties {
    /// Creates the properties used by one resolution pass.
    ///
    /// # Arguments
    ///
    /// * `application_title` - The title of the application, when known.
    /// * `application_version` - The version of the application, when known.
    /// * `default_value` - The value used for properties without a value.
    ///
    /// # Returns
    ///
    /// The properties of the banner.
    fn new(
        application_title: Option<&str>,
        application_version: Option<&str>,
        default_value: Option<&str>,
    ) -> Self {
        Self {
            application_title: application_title.map(str::to_string),
            versions: versions_map(application_version, default_value),
            default_value: default_value.map(str::to_string),
        }
    }

    /// Returns the value of the given property, if any.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the property.
    ///
    /// # Returns
    ///
    /// The value of the property, or `None` when it has none.
    fn get(&self, name: &str) -> Option<String> {
        if name == APPLICATION_TITLE_PROPERTY {
            return self
                .application_title
                .clone()
                .or_else(|| self.default_value.clone());
        }
        self.versions.get(name).cloned().flatten()
    }
}

/// Builds the version properties resolved by the banner.
///
/// # Arguments
///
/// * `application_version` - The version of the application, when known.
/// * `default_value` - The value used for unknown versions.
///
/// # Returns
///
/// The version properties, keyed by placeholder name.
fn versions_map(
    application_version: Option<&str>,
    default_value: Option<&str>,
) -> HashMap<&'static str, Option<String>> {
    let next_version = Some(NextWebVersion::get_version());
    HashMap::from([
        (
            APPLICATION_VERSION_PROPERTY,
            version_string(application_version, false, default_value),
        ),
        (
            NEXT_VERSION_PROPERTY,
            version_string(next_version, false, default_value),
        ),
        (
            APPLICATION_FORMATTED_VERSION_PROPERTY,
            version_string(application_version, true, default_value),
        ),
        (
            NEXT_FORMATTED_VERSION_PROPERTY,
            version_string(next_version, true, default_value),
        ),
    ])
}

/// Formats a version, falling back to `default_value` when it is unknown.
///
/// # Arguments
///
/// * `version` - The version, when known.
/// * `format` - Whether the version is wrapped into parentheses.
/// * `default_value` - The value used for an unknown version.
///
/// # Returns
///
/// The formatted version, or the default value.
fn version_string(
    version: Option<&str>,
    format: bool,
    default_value: Option<&str>,
) -> Option<String> {
    match version {
        None => default_value.map(str::to_string),
        Some(version) if format => Some(format!(" (v{version})")),
        Some(version) => Some(version.to_string()),
    }
}

/// Resolves the `${...}` placeholders of `text` using `lookup`.
///
/// The supported syntax is:
///
/// - `${key}` — replaced by the value of `key`, or kept verbatim when the key
///   cannot be resolved.
/// - `${key:default}` — replaced by the value of `key`, or by `default` when
///   the key cannot be resolved.
/// - Placeholders nested in a key, a value or a default value are resolved as
///   well, up to [`MAX_PLACEHOLDER_DEPTH`] levels.
///
/// # Arguments
///
/// * `text` - The text to resolve.
/// * `lookup` - The property lookup used for every placeholder key.
///
/// # Returns
///
/// The text with all resolvable placeholders replaced.
fn resolve_placeholders_with<F>(text: &str, lookup: &F) -> String
where
    F: Fn(&str) -> Option<String>,
{
    if !text.contains(PLACEHOLDER_PREFIX) {
        return text.to_string();
    }
    parse_placeholders(text, lookup, &mut Vec::new(), 0)
}

/// Resolves every placeholder of `text`, keeping the unresolvable ones.
///
/// # Arguments
///
/// * `text` - The text to resolve.
/// * `lookup` - The property lookup used for every placeholder key.
/// * `resolving` - The placeholders currently being resolved, used to detect
///   circular references.
/// * `depth` - The current nesting depth.
///
/// # Returns
///
/// The resolved text.
fn parse_placeholders<F>(
    text: &str,
    lookup: &F,
    resolving: &mut Vec<String>,
    depth: usize,
) -> String
where
    F: Fn(&str) -> Option<String>,
{
    if depth > MAX_PLACEHOLDER_DEPTH {
        warn!(
            depth,
            "placeholder nesting too deep, leaving the text unresolved"
        );
        return text.to_string();
    }

    let mut resolved = String::with_capacity(text.len());
    let mut cursor = 0;

    while let Some(offset) = text[cursor..].find(PLACEHOLDER_PREFIX) {
        let start = cursor + offset;
        // Without a closing suffix the rest of the text is kept verbatim.
        let Some(end) = find_placeholder_end(text, start) else {
            break;
        };

        resolved.push_str(&text[cursor..start]);
        let placeholder = &text[start + PLACEHOLDER_PREFIX.len()..end];
        match resolve_placeholder(placeholder, lookup, resolving, depth) {
            Some(value) => resolved.push_str(&value),
            None => resolved.push_str(&text[start..=end]),
        }
        cursor = end + 1;
    }

    resolved.push_str(&text[cursor..]);
    resolved
}

/// Resolves a single placeholder.
///
/// # Arguments
///
/// * `placeholder` - The placeholder, without its prefix and suffix.
/// * `lookup` - The property lookup used for the placeholder key.
/// * `resolving` - The placeholders currently being resolved.
/// * `depth` - The current nesting depth.
///
/// # Returns
///
/// The resolved value, or `None` when the placeholder cannot be resolved.
fn resolve_placeholder<F>(
    placeholder: &str,
    lookup: &F,
    resolving: &mut Vec<String>,
    depth: usize,
) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    if resolving.iter().any(|resolving| resolving == placeholder) {
        warn!(placeholder, "circular placeholder reference");
        return None;
    }

    resolving.push(placeholder.to_string());
    let value = lookup_placeholder(placeholder, lookup, resolving, depth);
    resolving.pop();

    value
}

/// Looks up the value of a placeholder, using its default value as a fallback.
///
/// # Arguments
///
/// * `placeholder` - The placeholder, without its prefix and suffix.
/// * `lookup` - The property lookup used for the placeholder key.
/// * `resolving` - The placeholders currently being resolved.
/// * `depth` - The current nesting depth.
///
/// # Returns
///
/// The value of the placeholder, or `None` when it cannot be resolved.
fn lookup_placeholder<F>(
    placeholder: &str,
    lookup: &F,
    resolving: &mut Vec<String>,
    depth: usize,
) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    // Placeholders nested in the key are resolved before the lookup.
    let key = parse_placeholders(placeholder, lookup, resolving, depth + 1);

    let value = lookup(&key).or_else(|| {
        // `${key:default}` — the default is used when the key has no value.
        let (key, default_value) = key.split_once(VALUE_SEPARATOR)?;
        lookup(key).or_else(|| Some(default_value.to_string()))
    })?;

    // Values and default values may contain placeholders of their own.
    Some(parse_placeholders(&value, lookup, resolving, depth + 1))
}

/// Returns the index of the `}` closing the placeholder starting at `start`.
///
/// Placeholders nested in the key are taken into account, so the closing suffix
/// of `${a-${b}}` is its last `}`.
///
/// # Arguments
///
/// * `text` - The text holding the placeholder.
/// * `start` - The index of the placeholder prefix.
///
/// # Returns
///
/// The index of the closing suffix, or `None` when the placeholder is not
/// closed.
fn find_placeholder_end(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut nested = 0;
    let mut index = start + PLACEHOLDER_PREFIX.len();

    while index < bytes.len() {
        match bytes[index] {
            b'$' if bytes.get(index + 1) == Some(&b'{') => {
                nested += 1;
                index += 2;
            }
            b'}' if nested == 0 => return Some(index),
            b'}' => {
                nested -= 1;
                index += 1;
            }
            _ => index += 1,
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns a lookup over the given properties.
    fn lookup<'a>(
        properties: &'a HashMap<&'a str, &'a str>,
    ) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| properties.get(name).map(|value| value.to_string())
    }

    #[test]
    fn version_string_falls_back_when_unknown() {
        assert_eq!(
            version_string(None, false, Some("fallback")),
            Some("fallback".to_string())
        );
        assert_eq!(version_string(None, false, None), None);
    }

    #[test]
    fn version_string_is_wrapped_when_formatted() {
        assert_eq!(
            version_string(Some("1.2.3"), true, None),
            Some(" (v1.2.3)".to_string())
        );
        assert_eq!(
            version_string(Some("1.2.3"), false, None),
            Some("1.2.3".to_string())
        );
    }

    #[test]
    fn versions_map_uses_an_empty_default_for_unknown_versions() {
        let versions = versions_map(None, Some(""));
        assert_eq!(
            versions.get(APPLICATION_VERSION_PROPERTY),
            Some(&Some(String::new()))
        );
        assert_eq!(
            versions.get(NEXT_VERSION_PROPERTY),
            Some(&Some(NextWebVersion::get_version().to_string()))
        );
    }

    #[test]
    fn versions_map_has_no_value_without_a_default() {
        let versions = versions_map(None, None);
        assert_eq!(versions.get(APPLICATION_VERSION_PROPERTY), Some(&None));
        assert_eq!(
            versions.get(NEXT_FORMATTED_VERSION_PROPERTY),
            Some(&Some(format!(" (v{})", NextWebVersion::get_version())))
        );
    }

    #[test]
    fn banner_properties_fall_back_to_the_default_value() {
        let empty = BannerProperties::new(None, None, Some(""));
        assert_eq!(empty.get(APPLICATION_TITLE_PROPERTY), Some(String::new()));

        let unset = BannerProperties::new(None, None, None);
        assert_eq!(unset.get(APPLICATION_TITLE_PROPERTY), None);

        let titled = BannerProperties::new(Some("Example"), None, None);
        assert_eq!(
            titled.get(APPLICATION_TITLE_PROPERTY),
            Some("Example".to_string())
        );
    }

    #[test]
    fn resolves_a_simple_placeholder() {
        let properties = HashMap::from([("application.title", "Example")]);
        assert_eq!(
            resolve_placeholders_with("${application.title}", &lookup(&properties)),
            "Example"
        );
    }

    #[test]
    fn keeps_an_unresolvable_placeholder() {
        let properties = HashMap::from([("known", "value")]);
        assert_eq!(
            resolve_placeholders_with("a ${unknown} ${known}", &lookup(&properties)),
            "a ${unknown} value"
        );
    }

    #[test]
    fn resolves_a_default_value() {
        let properties = HashMap::new();
        assert_eq!(
            resolve_placeholders_with("${unknown:fallback}", &lookup(&properties)),
            "fallback"
        );
        assert_eq!(
            resolve_placeholders_with("${unknown:}", &lookup(&properties)),
            ""
        );
    }

    #[test]
    fn prefers_the_property_over_its_default_value() {
        let properties = HashMap::from([("known", "value")]);
        assert_eq!(
            resolve_placeholders_with("${known:fallback}", &lookup(&properties)),
            "value"
        );
    }

    #[test]
    fn resolves_a_nested_key() {
        let properties = HashMap::from([("name", "title"), ("title", "Example")]);
        assert_eq!(
            resolve_placeholders_with("${${name}}", &lookup(&properties)),
            "Example"
        );
    }

    #[test]
    fn resolves_nested_value_and_default_value() {
        let properties = HashMap::from([("greeting", "${salutation}"), ("salutation", "Hello")]);
        assert_eq!(
            resolve_placeholders_with("${greeting}", &lookup(&properties)),
            "Hello"
        );
        assert_eq!(
            resolve_placeholders_with("${unknown:${salutation}}", &lookup(&properties)),
            "Hello"
        );
    }

    #[test]
    fn keeps_a_placeholder_referring_to_itself() {
        let properties = HashMap::from([("loop", "${loop}")]);
        assert_eq!(
            resolve_placeholders_with("${loop}", &lookup(&properties)),
            "${loop}"
        );
    }

    #[test]
    fn keeps_an_unclosed_placeholder() {
        let properties = HashMap::from([("known", "value")]);
        assert_eq!(
            resolve_placeholders_with("${known ${known}", &lookup(&properties)),
            "${known ${known}"
        );
    }

    #[test]
    fn resolves_every_placeholder_of_a_text() {
        let properties = HashMap::from([("a", "1"), ("b", "2")]);
        assert_eq!(
            resolve_placeholders_with("${a}-${b}", &lookup(&properties)),
            "1-2"
        );
    }

    #[test]
    fn finds_the_suffix_of_a_nested_placeholder() {
        assert_eq!(find_placeholder_end("${a-${b}}", 0), Some(8));
        assert_eq!(find_placeholder_end("${a}", 0), Some(3));
        assert_eq!(find_placeholder_end("${a", 0), None);
    }

}
