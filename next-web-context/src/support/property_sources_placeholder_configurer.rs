//! Port of `org.springframework.context.support.PropertySourcesPlaceholderConfigurer`.
//!
//! Resolves the `${...}` placeholders of configuration values, for example
//! `${next.server.port:8080}`, against a set of property sources:
//!
//! - the property sources of the environment, when one is
//!   [set](PropertySourcesPlaceholderConfigurer::set_environment),
//! - the local properties, that is the entries given to
//!   [`set_properties`](PropertySourcesPlaceholderConfigurer::set_properties)
//!   and the entries of the resources given to
//!   [`set_locations`](PropertySourcesPlaceholderConfigurer::set_locations).
//!
//! The local properties are searched after the ones of the environment, unless
//! [`local_override`](PropertySourcesPlaceholderConfigurer::set_local_override)
//! turns the precedence around. Calling
//! [`set_property_sources`](PropertySourcesPlaceholderConfigurer::set_property_sources)
//! replaces both, exactly like the original: once a caller decides which sources
//! are used, the configurer does not add any of its own.
//!
//! # Examples
//!
//! ```ignore
//! let mut configurer = PropertySourcesPlaceholderConfigurer::default();
//! configurer.set_properties(Properties::from_iter([("next.server.port", "8080")]));
//!
//! assert_eq!(
//!     configurer.resolve_placeholders("port is ${next.server.port:9090}"),
//!     "port is 8080"
//! );
//! ```

use std::fmt;
use std::io;
use std::sync::Arc;

use crate::support::BundleLoader;
use crate::util::Properties;

/// The name of the [`PropertySource`] the merged local properties are added
/// under.
///
/// Equivalent to `LOCAL_PROPERTIES_PROPERTY_SOURCE_NAME`.
pub const LOCAL_PROPERTIES_PROPERTY_SOURCE_NAME: &str = "localProperties";

/// The name of the [`PropertySource`] the environment is added under.
///
/// Equivalent to `ENVIRONMENT_PROPERTIES_PROPERTY_SOURCE_NAME`.
pub const ENVIRONMENT_PROPERTIES_PROPERTY_SOURCE_NAME: &str = "environmentProperties";

/// The prefix that opens a placeholder.
pub const DEFAULT_PLACEHOLDER_PREFIX: &str = "${";

/// The suffix that closes a placeholder.
pub const DEFAULT_PLACEHOLDER_SUFFIX: &str = "}";

/// The text that separates the key of a placeholder from its default value.
pub const DEFAULT_VALUE_SEPARATOR: &str = ":";

/// The character that turns the placeholder prefix that follows it into text.
pub const DEFAULT_ESCAPE_CHARACTER: char = '\\';

/// The maximum number of nested placeholders that are resolved.
const MAX_PLACEHOLDER_DEPTH: usize = 8;

/// A named source of properties.
///
/// The trait is the counterpart of Spring's `PropertySource`: this crate sits
/// below the crate that owns the environment, so the configurer looks its
/// properties up through this trait and an application adapts its own
/// environment to it. [`BundleLoader`] plays the same role for the resources of
/// a bundle.
pub trait PropertySource: fmt::Debug + Send + Sync {
    /// Returns the name of this source.
    ///
    /// The name identifies the source in the collection the configurer applies.
    fn name(&self) -> &str;

    /// Returns the value of the given property, or `None` when this source does
    /// not define it.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the property to look up.
    fn property(&self, name: &str) -> Option<String>;

    /// Returns whether this source defines the given property.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the property to look up.
    fn contains_property(&self, name: &str) -> bool {
        self.property(name).is_some()
    }

    /// Returns the names of the properties of this source.
    ///
    /// The default implementation returns an empty list, since a source does
    /// not have to be enumerable.
    fn property_names(&self) -> Vec<String> {
        Vec::new()
    }
}

/// A [`PropertySource`] backed by a [`Properties`] map.
///
/// Equivalent to Spring's `PropertiesPropertySource`: it is the source the
/// configurer adds for its local properties, and the one an application uses
/// for the properties it holds in memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapPropertySource {
    name: String,
    properties: Properties,
}

impl MapPropertySource {
    /// Creates a source with the given name over the given properties.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the source.
    /// * `properties` - The entries of the source.
    pub fn new(name: impl Into<String>, properties: Properties) -> Self {
        Self {
            name: name.into(),
            properties,
        }
    }

    /// Returns the entries of this source.
    pub fn properties(&self) -> &Properties {
        &self.properties
    }

    /// Returns the entries of this source, for modification.
    pub fn properties_mut(&mut self) -> &mut Properties {
        &mut self.properties
    }
}

impl PropertySource for MapPropertySource {
    fn name(&self) -> &str {
        &self.name
    }

    fn property(&self, name: &str) -> Option<String> {
        self.properties.get_property(name).map(str::to_owned)
    }

    fn property_names(&self) -> Vec<String> {
        self.properties
            .keys()
            .into_iter()
            .map(str::to_owned)
            .collect()
    }
}

/// Error returned when a placeholder has to be resolved and cannot be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceholderResolutionError {
    placeholder: String,
}

impl PlaceholderResolutionError {
    /// Creates the error of the given placeholder.
    fn new(placeholder: impl Into<String>) -> Self {
        Self {
            placeholder: placeholder.into(),
        }
    }

    /// Returns the text between the placeholder markers that could not be
    /// resolved.
    pub fn placeholder(&self) -> &str {
        &self.placeholder
    }
}

impl fmt::Display for PlaceholderResolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Could not resolve placeholder '{}' in a configuration value",
            self.placeholder
        )
    }
}

impl std::error::Error for PlaceholderResolutionError {}

/// Resolves `${...}` placeholders against a set of [`PropertySource`]s.
///
/// The type is the Rust counterpart of
/// `org.springframework.context.support.PropertySourcesPlaceholderConfigurer`,
/// together with the configuration of `PlaceholderConfigurerSupport` and the
/// resolution of `PropertySourcesPropertyResolver`.
///
/// It is used in two steps:
///
/// 1. the configuration is given to it, through the setters it exposes;
/// 2. [`apply`](Self::apply) assembles the sources it resolves against and
///    records them, after which the resolution methods can be used.
///
/// Resolution also works before [`apply`](Self::apply) was called, in which
/// case it searches the environment and then the local properties that were
/// given to the configurer in memory. The resources of the configured locations
/// are only read by [`apply`](Self::apply), since reading them can fail.
pub struct PropertySourcesPlaceholderConfigurer {
    placeholder_prefix: String,
    placeholder_suffix: String,
    value_separator: Option<String>,
    escape_character: Option<char>,
    trim_values: bool,
    null_value: Option<String>,
    ignore_unresolvable_placeholders: bool,
    local_override: bool,
    properties: Properties,
    locations: Vec<String>,
    bundle_loader: Option<Arc<dyn BundleLoader>>,
    environment: Option<Arc<dyn PropertySource>>,
    property_sources: Option<Vec<Arc<dyn PropertySource>>>,
    applied_property_sources: Option<Vec<Arc<dyn PropertySource>>>,
}

impl PropertySourcesPlaceholderConfigurer {
    /// Returns the prefix that opens a placeholder.
    pub fn placeholder_prefix(&self) -> &str {
        &self.placeholder_prefix
    }

    /// Sets the prefix that opens a placeholder.
    ///
    /// # Arguments
    ///
    /// * `placeholder_prefix` - The prefix, `${` by default.
    pub fn set_placeholder_prefix(&mut self, placeholder_prefix: impl Into<String>) {
        self.placeholder_prefix = placeholder_prefix.into();
    }

    /// Returns the suffix that closes a placeholder.
    pub fn placeholder_suffix(&self) -> &str {
        &self.placeholder_suffix
    }

    /// Sets the suffix that closes a placeholder.
    ///
    /// # Arguments
    ///
    /// * `placeholder_suffix` - The suffix, `}` by default.
    pub fn set_placeholder_suffix(&mut self, placeholder_suffix: impl Into<String>) {
        self.placeholder_suffix = placeholder_suffix.into();
    }

    /// Returns the separator between the key of a placeholder and its default
    /// value.
    pub fn value_separator(&self) -> Option<&str> {
        self.value_separator.as_deref()
    }

    /// Sets the separator between the key of a placeholder and its default
    /// value.
    ///
    /// # Arguments
    ///
    /// * `value_separator` - The separator, `:` by default.
    pub fn set_value_separator(&mut self, value_separator: impl Into<String>) {
        self.value_separator = Some(value_separator.into());
    }

    /// Clears the separator between the key of a placeholder and its default
    /// value.
    ///
    /// A placeholder then only names its key, and a `:` inside it is part of
    /// that key.
    pub fn clear_value_separator(&mut self) {
        self.value_separator = None;
    }

    /// Returns the character that turns the prefix that follows it into text.
    pub fn escape_character(&self) -> Option<char> {
        self.escape_character
    }

    /// Sets the character that turns the prefix that follows it into text, so
    /// that `${a}` is resolved while `\${a}` stays as it is written.
    ///
    /// # Arguments
    ///
    /// * `escape_character` - The escape character, `\` by default.
    pub fn set_escape_character(&mut self, escape_character: char) {
        self.escape_character = Some(escape_character);
    }

    /// Clears the character that escapes the placeholder prefix.
    pub fn clear_escape_character(&mut self) {
        self.escape_character = None;
    }

    /// Returns whether the resolved values are trimmed.
    pub fn is_trim_values(&self) -> bool {
        self.trim_values
    }

    /// Sets whether the resolved values are trimmed.
    ///
    /// # Arguments
    ///
    /// * `trim_values` - Whether the surrounding whitespace of a resolved value
    ///   is removed.
    pub fn set_trim_values(&mut self, trim_values: bool) {
        self.trim_values = trim_values;
    }

    /// Returns the value that makes a resolved value null.
    pub fn null_value(&self) -> Option<&str> {
        self.null_value.as_deref()
    }

    /// Sets the value that makes a resolved value null, an empty string for
    /// example.
    ///
    /// # Arguments
    ///
    /// * `null_value` - The text a resolved value is compared against.
    pub fn set_null_value(&mut self, null_value: impl Into<String>) {
        self.null_value = Some(null_value.into());
    }

    /// Clears the value that makes a resolved value null.
    pub fn clear_null_value(&mut self) {
        self.null_value = None;
    }

    /// Returns whether a placeholder that cannot be resolved is disregarded.
    pub fn is_ignore_unresolvable_placeholders(&self) -> bool {
        self.ignore_unresolvable_placeholders
    }

    /// Sets whether a placeholder that cannot be resolved is disregarded.
    ///
    /// When this is turned on, resolving a value that holds a placeholder
    /// without a value keeps the placeholder as it is written instead of
    /// reporting it as an error.
    ///
    /// # Arguments
    ///
    /// * `ignore_unresolvable_placeholders` - Whether an unresolvable
    ///   placeholder is disregarded.
    pub fn set_ignore_unresolvable_placeholders(&mut self, ignore_unresolvable_placeholders: bool) {
        self.ignore_unresolvable_placeholders = ignore_unresolvable_placeholders;
    }

    /// Returns whether the local properties take precedence over the ones of
    /// the environment.
    pub fn is_local_override(&self) -> bool {
        self.local_override
    }

    /// Sets whether the local properties take precedence over the ones of the
    /// environment.
    ///
    /// Equivalent to `setLocalOverride(boolean)`. The default is `false`, which
    /// means that the local properties are searched last, after the property
    /// sources of the environment.
    ///
    /// # Arguments
    ///
    /// * `local_override` - Whether the local properties win.
    pub fn set_local_override(&mut self, local_override: bool) {
        self.local_override = local_override;
    }

    /// Returns the local properties, without the ones of the configured
    /// locations.
    pub fn properties(&self) -> &Properties {
        &self.properties
    }

    /// Replaces the local properties.
    ///
    /// Equivalent to `setProperties(Properties)`.
    ///
    /// # Arguments
    ///
    /// * `properties` - The entries the configurer adds as its local
    ///   properties.
    pub fn set_properties(&mut self, properties: Properties) {
        self.properties = properties;
    }

    /// Returns the locations the local properties are read from.
    pub fn locations(&self) -> &[String] {
        &self.locations
    }

    /// Replaces the locations the local properties are read from.
    ///
    /// Equivalent to `setLocations(Resource...)`. The locations are read by
    /// [`apply`](Self::apply) with the configured [`BundleLoader`], as UTF-8
    /// properties files.
    ///
    /// # Arguments
    ///
    /// * `locations` - The locations of the properties files.
    pub fn set_locations<I>(&mut self, locations: I)
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        self.locations = locations.into_iter().map(Into::into).collect();
    }

    /// Adds a location the local properties are read from.
    ///
    /// Equivalent to `setLocation(Resource)`.
    ///
    /// # Arguments
    ///
    /// * `location` - The location of a properties file.
    pub fn add_location(&mut self, location: impl Into<String>) {
        self.locations.push(location.into());
    }

    /// Returns the loader the configured locations are read with.
    pub fn bundle_loader(&self) -> Option<&Arc<dyn BundleLoader>> {
        self.bundle_loader.as_ref()
    }

    /// Sets the loader the configured locations are read with.
    ///
    /// The loader is the same one a message source reads its bundles with, and
    /// it is only needed when locations are configured.
    ///
    /// # Arguments
    ///
    /// * `bundle_loader` - The loader of the properties files.
    pub fn set_bundle_loader(&mut self, bundle_loader: Arc<dyn BundleLoader>) {
        self.bundle_loader = Some(bundle_loader);
    }

    /// Returns the environment the properties are looked up in.
    pub fn environment(&self) -> Option<&Arc<dyn PropertySource>> {
        self.environment.as_ref()
    }

    /// Sets the environment the properties are looked up in.
    ///
    /// Equivalent to `setEnvironment(Environment)`: the environment is
    /// searched, as a single source named
    /// [`ENVIRONMENT_PROPERTIES_PROPERTY_SOURCE_NAME`], before the local
    /// properties.
    ///
    /// # Arguments
    ///
    /// * `environment` - The source the properties of the environment are read
    ///   from. An application adapts its own environment to a
    ///   [`PropertySource`].
    pub fn set_environment(&mut self, environment: Arc<dyn PropertySource>) {
        self.environment = Some(environment);
    }

    /// Returns the sources that replace the environment and the local
    /// properties.
    pub fn property_sources(&self) -> Option<&[Arc<dyn PropertySource>]> {
        self.property_sources.as_deref()
    }

    /// Replaces the sources the configurer resolves against.
    ///
    /// Equivalent to `setPropertySources(PropertySources)`: setting the sources
    /// indicates that the environment and the local properties are to be
    /// ignored.
    ///
    /// # Arguments
    ///
    /// * `property_sources` - The sources, from the highest to the lowest
    ///   precedence.
    pub fn set_property_sources<I>(&mut self, property_sources: I)
    where
        I: IntoIterator<Item = Arc<dyn PropertySource>>,
    {
        self.property_sources = Some(property_sources.into_iter().collect());
    }

    /// Returns the sources that were applied, or `None` when the configurer was
    /// not applied yet.
    ///
    /// Equivalent to `getAppliedPropertySources()`, which reports the sources
    /// that a post-processing applied.
    pub fn applied_property_sources(&self) -> Option<&[Arc<dyn PropertySource>]> {
        self.applied_property_sources.as_deref()
    }

    /// Returns whether the configurer was applied.
    pub fn is_applied(&self) -> bool {
        self.applied_property_sources.is_some()
    }

    /// Assembles the sources the configurer resolves against, and records them.
    ///
    /// Equivalent to `postProcessBeanFactory(ConfigurableListableBeanFactory)`
    /// without a bean factory: it reads the resources of the configured
    /// locations, and it decides the precedence of the local properties. When
    /// [`set_property_sources`](Self::set_property_sources) was called, the
    /// given sources are used as they are and nothing is added.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] when a resource of a configured location does
    /// not exist or cannot be read, or when locations are configured without a
    /// loader.
    pub fn apply(&mut self) -> io::Result<()> {
        if self.property_sources.is_none() {
            let mut property_sources = Vec::new();

            if let Some(environment) = self.environment.as_ref() {
                property_sources.push(Arc::new(NamedPropertySource::new(
                    ENVIRONMENT_PROPERTIES_PROPERTY_SOURCE_NAME,
                    Arc::clone(environment),
                )) as Arc<dyn PropertySource>);
            }

            let local_properties = Arc::new(MapPropertySource::new(
                LOCAL_PROPERTIES_PROPERTY_SOURCE_NAME,
                self.merge_properties()?,
            )) as Arc<dyn PropertySource>;

            if self.local_override {
                property_sources.insert(0, local_properties);
            } else {
                property_sources.push(local_properties);
            }

            self.property_sources = Some(property_sources);
        }

        self.applied_property_sources = self.property_sources.clone();

        Ok(())
    }

    /// Merges the local properties with the properties of the configured
    /// locations.
    ///
    /// Equivalent to `mergeProperties()`. When
    /// [localOverride](Self::set_local_override) is turned on, the entries of
    /// the local properties win over the entries of the resources, and the
    /// other way around otherwise.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] when a resource of a configured location cannot
    /// be read.
    pub fn merge_properties(&self) -> io::Result<Properties> {
        let mut merged = Properties::new();

        if self.local_override {
            self.load_locations(&mut merged)?;
            merged.put_all(&self.properties);
        } else {
            merged.put_all(&self.properties);
            self.load_locations(&mut merged)?;
        }

        Ok(merged)
    }

    /// Returns whether the given property is available.
    ///
    /// Equivalent to `containsProperty(String)`.
    ///
    /// # Arguments
    ///
    /// * `key` - The name of the property.
    pub fn contains_property(&self, key: &str) -> bool {
        self.get_property(key).is_some()
    }

    /// Returns the value of the given property, or `None` when it is not
    /// available.
    ///
    /// Equivalent to `getProperty(String)`: the value is looked up in the
    /// applied sources, or, before the configurer was applied, in the
    /// environment and then in the local properties.
    ///
    /// # Arguments
    ///
    /// * `key` - The name of the property.
    pub fn get_property(&self, key: &str) -> Option<String> {
        if let Some(property_sources) = self.applied_property_sources.as_deref() {
            return property_sources
                .iter()
                .find_map(|property_source| property_source.property(key));
        }

        if let Some(value) = self
            .environment
            .as_deref()
            .and_then(|environment| environment.property(key))
        {
            return Some(value);
        }

        self.properties.get_property(key).map(str::to_owned)
    }

    /// Resolves the placeholders of the given text.
    ///
    /// A placeholder that cannot be resolved is kept as it is written, which is
    /// how the configurer behaves when
    /// [ignoreUnresolvablePlaceholders](Self::set_ignore_unresolvable_placeholders)
    /// is turned on.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to resolve the placeholders of.
    pub fn resolve_placeholders(&self, text: &str) -> String {
        self.resolve_text(text, false)
            .unwrap_or_else(|_| text.to_owned())
    }

    /// Resolves the placeholders of the given text, reporting the first
    /// placeholder that cannot be resolved.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to resolve the placeholders of.
    ///
    /// # Errors
    ///
    /// Returns a [`PlaceholderResolutionError`] when a placeholder has no value
    /// and declares no default value.
    pub fn resolve_required_placeholders(
        &self,
        text: &str,
    ) -> Result<String, PlaceholderResolutionError> {
        self.resolve_text(text, true)
    }

    /// Resolves the value of the given text, the way a configuration value is
    /// resolved.
    ///
    /// Equivalent to the value resolver of
    /// `processProperties(ConfigurableListableBeanFactory, ConfigurablePropertyResolver)`:
    /// the placeholders are resolved, the result is trimmed when
    /// [trimValues](Self::set_trim_values) is turned on, and a result that
    /// equals the configured [nullValue](Self::set_null_value) resolves to
    /// `None`.
    ///
    /// # Arguments
    ///
    /// * `text` - The value to resolve.
    ///
    /// # Errors
    ///
    /// Returns a [`PlaceholderResolutionError`] when a required placeholder
    /// cannot be resolved.
    pub fn resolve_value(&self, text: &str) -> Result<Option<String>, PlaceholderResolutionError> {
        let resolved = if self.ignore_unresolvable_placeholders {
            self.resolve_placeholders(text)
        } else {
            self.resolve_required_placeholders(text)?
        };

        let resolved = if self.trim_values {
            resolved.trim().to_owned()
        } else {
            resolved
        };

        match self.null_value.as_deref() {
            Some(null_value) if null_value == resolved => Ok(None),
            _ => Ok(Some(resolved)),
        }
    }

    /// Reads the configured locations into the given properties.
    ///
    /// # Arguments
    ///
    /// * `properties` - The instance the entries of the locations are added to.
    fn load_locations(&self, properties: &mut Properties) -> io::Result<()> {
        if self.locations.is_empty() {
            return Ok(());
        }

        let Some(bundle_loader) = self.bundle_loader.as_deref() else {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "No bundle loader is configured for the locations of the configurer",
            ));
        };

        for location in &self.locations {
            let content = bundle_loader.load(location)?.ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Failed to load properties from '{location}': resource not found"),
                )
            })?;

            properties.load_from_bytes(&content)?;
        }

        Ok(())
    }

    /// Resolves the placeholders of the given text.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to resolve the placeholders of.
    /// * `required` - Whether a placeholder without a value is an error.
    fn resolve_text(
        &self,
        text: &str,
        required: bool,
    ) -> Result<String, PlaceholderResolutionError> {
        self.resolve_at_depth(text, required, 0)
    }

    /// Resolves the placeholders of the given text.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to resolve the placeholders of.
    /// * `required` - Whether a placeholder without a value is an error.
    /// * `depth` - The current nesting depth, used to stop the recursion.
    fn resolve_at_depth(
        &self,
        text: &str,
        required: bool,
        depth: usize,
    ) -> Result<String, PlaceholderResolutionError> {
        let mut resolved = String::with_capacity(text.len());
        let mut index = 0;

        while index < text.len() {
            let remaining = &text[index..];

            let Some(found) = remaining.find(self.placeholder_prefix.as_str()) else {
                resolved.push_str(remaining);
                break;
            };
            let placeholder_start = index + found;

            // An escape character turns the prefix that follows it into text.
            if let Some(escape_character) = self.escape_character {
                let escape_length = escape_character.len_utf8();
                if found >= escape_length && remaining[..found].ends_with(escape_character) {
                    resolved.push_str(&remaining[..found - escape_length]);
                    resolved.push_str(&self.placeholder_prefix);
                    index = placeholder_start + self.placeholder_prefix.len();
                    continue;
                }
            }

            resolved.push_str(&remaining[..found]);

            let after_prefix = &text[placeholder_start + self.placeholder_prefix.len()..];
            let Some(end) = self.find_placeholder_end(after_prefix) else {
                // Without a suffix the rest of the text is not a placeholder.
                resolved.push_str(&text[placeholder_start..]);
                break;
            };

            let placeholder = &after_prefix[..end];
            let placeholder_length =
                self.placeholder_prefix.len() + end + self.placeholder_suffix.len();
            let original = &text[placeholder_start..placeholder_start + placeholder_length];

            let value = if depth < MAX_PLACEHOLDER_DEPTH {
                self.resolve_placeholder(placeholder, required, depth)?
            } else {
                None
            };

            match value {
                Some(value) => resolved.push_str(&value),
                None if required => {
                    return Err(PlaceholderResolutionError::new(placeholder));
                }
                None => resolved.push_str(original),
            }

            index = placeholder_start + placeholder_length;
        }

        Ok(resolved)
    }

    /// Resolves a single placeholder.
    ///
    /// The nested placeholders of the placeholder are resolved first, so that
    /// `${${key.name}}` and `${key:${fallback}}` both work. The text between the
    /// placeholder markers is tried as a key; when it holds a value separator,
    /// the text before the separator is tried as a key as well, and the text
    /// after it is the default value.
    ///
    /// # Arguments
    ///
    /// * `placeholder` - The text between the placeholder markers.
    /// * `required` - Whether a placeholder without a value is an error.
    /// * `depth` - The current nesting depth, used to stop the recursion.
    fn resolve_placeholder(
        &self,
        placeholder: &str,
        required: bool,
        depth: usize,
    ) -> Result<Option<String>, PlaceholderResolutionError> {
        let placeholder = self.resolve_at_depth(placeholder, required, depth + 1)?;

        if let Some(value) = self.get_property(&placeholder) {
            return self.resolve_at_depth(&value, required, depth + 1).map(Some);
        }

        let Some(value_separator) = self.value_separator.as_deref() else {
            return Ok(None);
        };

        let Some((key, default_value)) = placeholder.split_once(value_separator) else {
            return Ok(None);
        };

        if let Some(value) = self.get_property(key) {
            return self.resolve_at_depth(&value, required, depth + 1).map(Some);
        }

        Ok(Some(default_value.to_owned()))
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

impl fmt::Debug for PropertySourcesPlaceholderConfigurer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PropertySourcesPlaceholderConfigurer")
            .field("placeholder_prefix", &self.placeholder_prefix)
            .field("placeholder_suffix", &self.placeholder_suffix)
            .field("value_separator", &self.value_separator)
            .field("escape_character", &self.escape_character)
            .field("trim_values", &self.trim_values)
            .field("null_value", &self.null_value)
            .field(
                "ignore_unresolvable_placeholders",
                &self.ignore_unresolvable_placeholders,
            )
            .field("local_override", &self.local_override)
            .field("properties", &self.properties)
            .field("locations", &self.locations)
            .field("environment", &self.environment.is_some())
            .field("applied", &self.is_applied())
            .finish()
    }
}

impl Clone for PropertySourcesPlaceholderConfigurer {
    fn clone(&self) -> Self {
        Self {
            placeholder_prefix: self.placeholder_prefix.clone(),
            placeholder_suffix: self.placeholder_suffix.clone(),
            value_separator: self.value_separator.clone(),
            escape_character: self.escape_character,
            trim_values: self.trim_values,
            null_value: self.null_value.clone(),
            ignore_unresolvable_placeholders: self.ignore_unresolvable_placeholders,
            local_override: self.local_override,
            properties: self.properties.clone(),
            locations: self.locations.clone(),
            bundle_loader: self.bundle_loader.clone(),
            environment: self.environment.clone(),
            property_sources: self.property_sources.clone(),
            applied_property_sources: self.applied_property_sources.clone(),
        }
    }
}

impl Default for PropertySourcesPlaceholderConfigurer {
    fn default() -> Self {
        Self {
            placeholder_prefix: DEFAULT_PLACEHOLDER_PREFIX.to_owned(),
            placeholder_suffix: DEFAULT_PLACEHOLDER_SUFFIX.to_owned(),
            value_separator: Some(DEFAULT_VALUE_SEPARATOR.to_owned()),
            escape_character: Some(DEFAULT_ESCAPE_CHARACTER),
            trim_values: false,
            null_value: None,
            ignore_unresolvable_placeholders: false,
            local_override: false,
            properties: Properties::new(),
            locations: Vec::new(),
            bundle_loader: None,
            environment: None,
            property_sources: None,
            applied_property_sources: None,
        }
    }
}

/// A [`PropertySource`] that delegates to another one under a different name.
///
/// The configurer uses it to record the environment it was given under
/// [`ENVIRONMENT_PROPERTIES_PROPERTY_SOURCE_NAME`], the way the original wraps
/// an environment in a `PropertySource` of that name.
struct NamedPropertySource {
    name: String,
    source: Arc<dyn PropertySource>,
}

impl NamedPropertySource {
    /// Creates a source with the given name over the given source.
    fn new(name: impl Into<String>, source: Arc<dyn PropertySource>) -> Self {
        Self {
            name: name.into(),
            source,
        }
    }
}

impl PropertySource for NamedPropertySource {
    fn name(&self) -> &str {
        &self.name
    }

    fn property(&self, name: &str) -> Option<String> {
        self.source.property(name)
    }

    fn contains_property(&self, name: &str) -> bool {
        self.source.contains_property(name)
    }

    fn property_names(&self) -> Vec<String> {
        self.source.property_names()
    }
}

impl fmt::Debug for NamedPropertySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NamedPropertySource")
            .field("name", &self.name)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// A property source over a map of properties.
    #[derive(Debug)]
    struct TestPropertySource {
        name: String,
        properties: HashMap<String, String>,
    }

    impl TestPropertySource {
        fn new(name: &str, properties: &[(&str, &str)]) -> Arc<Self> {
            Arc::new(Self {
                name: name.to_owned(),
                properties: properties
                    .iter()
                    .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                    .collect(),
            })
        }
    }

    impl PropertySource for TestPropertySource {
        fn name(&self) -> &str {
            &self.name
        }

        fn property(&self, name: &str) -> Option<String> {
            self.properties.get(name).cloned()
        }
    }

    /// A loader that serves the properties files it is given.
    #[derive(Debug)]
    struct InMemoryBundleLoader(HashMap<String, String>);

    impl InMemoryBundleLoader {
        fn new(entries: &[(&str, &str)]) -> Arc<Self> {
            Arc::new(Self(
                entries
                    .iter()
                    .map(|(location, content)| ((*location).to_owned(), (*content).to_owned()))
                    .collect(),
            ))
        }
    }

    impl BundleLoader for InMemoryBundleLoader {
        fn load(&self, location: &str) -> io::Result<Option<Vec<u8>>> {
            Ok(self
                .0
                .get(location)
                .map(|content| content.as_bytes().to_vec()))
        }
    }

    /// Returns a configurer over the given local properties.
    fn configurer(properties: &[(&str, &str)]) -> PropertySourcesPlaceholderConfigurer {
        let mut configurer = PropertySourcesPlaceholderConfigurer::default();
        configurer.set_properties(Properties::from_iter(properties.iter().copied()));
        configurer
    }

    #[test]
    fn resolves_the_placeholders_of_its_properties() {
        let configurer = configurer(&[("next.server.port", "8080")]);

        assert_eq!(
            configurer.resolve_placeholders("the port is ${next.server.port}"),
            "the port is 8080"
        );
        assert_eq!(
            configurer.resolve_placeholders("a plain value"),
            "a plain value"
        );
        assert_eq!(
            configurer.resolve_placeholders("an unclosed ${next.server.port"),
            "an unclosed ${next.server.port"
        );
    }

    #[test]
    fn resolves_the_default_value_of_a_placeholder() {
        let configurer = configurer(&[]);

        assert_eq!(
            configurer.resolve_placeholders("${next.server.port:8080}"),
            "8080"
        );
        assert_eq!(
            configurer.resolve_placeholders("${next.server.port}"),
            "${next.server.port}"
        );
    }

    #[test]
    fn resolves_nested_placeholders() {
        let configurer = configurer(&[
            ("next.name", "demo"),
            ("next.description", "the ${next.name} application"),
        ]);

        assert_eq!(
            configurer.resolve_placeholders("${next.description}"),
            "the demo application"
        );
        assert_eq!(
            configurer.resolve_placeholders("${next.missing:${next.name}}"),
            "demo"
        );
    }

    #[test]
    fn escapes_the_placeholder_prefix() {
        let escaped = configurer(&[("next.name", "demo")]);

        assert_eq!(
            escaped.resolve_placeholders(r"\${next.name}"),
            "${next.name}"
        );
        assert_eq!(
            escaped.resolve_placeholders(r"${next.name} and \${next.name}"),
            "demo and ${next.name}"
        );

        let mut unescaped = configurer(&[("next.name", "demo")]);
        unescaped.clear_escape_character();
        assert_eq!(unescaped.resolve_placeholders(r"\${next.name}"), r"\demo");
    }

    #[test]
    fn resolves_placeholders_with_other_markers() {
        let mut configurer = configurer(&[("next.server.port", "8080")]);
        configurer.set_placeholder_prefix("%{");

        assert_eq!(
            configurer.resolve_placeholders("the port is %{next.server.port:9090}"),
            "the port is 8080"
        );
        assert_eq!(
            configurer.resolve_placeholders("${next.server.port}"),
            "${next.server.port}"
        );

        configurer.clear_value_separator();
        assert_eq!(
            configurer.resolve_placeholders("%{next.server.port:9090}"),
            "%{next.server.port:9090}"
        );
    }

    #[test]
    fn reports_a_required_placeholder_that_cannot_be_resolved() {
        let configurer = configurer(&[]);

        let error = configurer
            .resolve_required_placeholders("the port is ${next.server.port}")
            .unwrap_err();

        assert_eq!(error.placeholder(), "next.server.port");
        assert_eq!(
            configurer
                .resolve_required_placeholders("the port is ${next.server.port:8080}")
                .unwrap(),
            "the port is 8080"
        );
    }

    #[test]
    fn trims_the_resolved_value_and_reports_the_null_value() {
        let mut configurer = configurer(&[("next.name", "  demo  "), ("next.empty", " ")]);
        configurer.set_trim_values(true);
        configurer.set_null_value("");

        assert_eq!(
            configurer.resolve_value("${next.name}").unwrap(),
            Some("demo".to_owned())
        );
        assert_eq!(configurer.resolve_value("${next.empty}").unwrap(), None);
        assert_eq!(
            configurer.get_property("next.name"),
            Some("  demo  ".to_owned())
        );
    }

    #[test]
    fn disregards_an_unresolvable_placeholder_when_asked_to() {
        let mut configurer = configurer(&[]);

        assert!(configurer.resolve_value("${next.server.port}").is_err());

        configurer.set_ignore_unresolvable_placeholders(true);
        assert_eq!(
            configurer.resolve_value("${next.server.port}").unwrap(),
            Some("${next.server.port}".to_owned())
        );
    }

    #[test]
    fn applies_the_environment_before_the_local_properties() {
        let mut configurer = configurer(&[("next.name", "local"), ("next.port", "8080")]);
        configurer.set_environment(TestPropertySource::new(
            "environment",
            &[("next.name", "environment")],
        ));

        assert!(!configurer.is_applied());
        configurer.apply().unwrap();
        assert!(configurer.is_applied());

        assert_eq!(
            configurer.get_property("next.name"),
            Some("environment".to_owned())
        );
        assert_eq!(
            configurer.get_property("next.port"),
            Some("8080".to_owned())
        );

        let applied = configurer.applied_property_sources().unwrap();
        assert_eq!(applied.len(), 2);
        assert_eq!(
            applied[0].name(),
            ENVIRONMENT_PROPERTIES_PROPERTY_SOURCE_NAME
        );
        assert_eq!(applied[1].name(), LOCAL_PROPERTIES_PROPERTY_SOURCE_NAME);
    }

    #[test]
    fn applies_the_local_properties_first_when_they_override_the_environment() {
        let mut configurer = configurer(&[("next.name", "local")]);
        configurer.set_environment(TestPropertySource::new(
            "environment",
            &[("next.name", "environment")],
        ));
        configurer.set_local_override(true);

        configurer.apply().unwrap();

        assert_eq!(
            configurer.get_property("next.name"),
            Some("local".to_owned())
        );

        let applied = configurer.applied_property_sources().unwrap();
        assert_eq!(applied[0].name(), LOCAL_PROPERTIES_PROPERTY_SOURCE_NAME);
        assert_eq!(
            applied[1].name(),
            ENVIRONMENT_PROPERTIES_PROPERTY_SOURCE_NAME
        );
    }

    #[test]
    fn applies_the_properties_of_the_configured_locations() {
        let mut configurer = configurer(&[("next.name", "local")]);
        configurer.set_bundle_loader(InMemoryBundleLoader::new(&[(
            "config/next.properties",
            "next.name=from the resource\nnext.port=8080\n",
        )]));
        configurer.set_locations(["config/next.properties"]);

        assert_eq!(
            configurer
                .merge_properties()
                .unwrap()
                .get_property("next.name"),
            Some("from the resource")
        );

        configurer.apply().unwrap();
        assert_eq!(
            configurer.get_property("next.name"),
            Some("from the resource".to_owned())
        );

        configurer.set_local_override(true);
        assert_eq!(
            configurer
                .merge_properties()
                .unwrap()
                .get_property("next.name"),
            Some("local")
        );
    }

    #[test]
    fn reports_a_location_that_cannot_be_read() {
        let mut configurer = configurer(&[]);
        configurer.set_locations(["config/next.properties"]);

        assert_eq!(
            configurer.apply().unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );

        configurer.set_bundle_loader(InMemoryBundleLoader::new(&[]));
        assert_eq!(
            configurer.apply().unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }

    #[test]
    fn uses_the_given_sources_instead_of_the_environment_and_the_local_properties() {
        let mut configurer = configurer(&[("next.name", "local")]);
        configurer.set_environment(TestPropertySource::new(
            "environment",
            &[("next.name", "environment")],
        ));
        configurer
            .set_property_sources([TestPropertySource::new("given", &[("next.port", "9090")])
                as Arc<dyn PropertySource>]);

        configurer.apply().unwrap();

        assert_eq!(
            configurer.get_property("next.port"),
            Some("9090".to_owned())
        );
        assert_eq!(configurer.get_property("next.name"), None);
        assert_eq!(
            configurer.applied_property_sources().unwrap()[0].name(),
            "given"
        );
    }
}
