//! Port of `org.springframework.context.support.ResourceBundleMessageSource`.
//!
//! A message source that reads its messages from property bundles named after a
//! basename:
//!
//! ```text
//! messages/messages.properties
//! messages/messages_zh.properties
//! messages/messages_zh_CN.properties
//! ```
//!
//! The bundles are read as UTF-8 and are cached the way
//! [`cache_duration`](ResourceBundleMessageSource::set_cache_duration) asks for.
//! Messages are rendered with the argument syntax of
//! [`MessageFormat`](crate::util::MessageFormat), and a lookup that misses the
//! bundle of the requested locale falls back to the locale independent bundle,
//! to the [default locale](ResourceBundleMessageSource::set_default_locale) or
//! to the [locale of the
//! system](ResourceBundleMessageSource::set_fallback_to_system_locale).
//!
//! # Examples
//!
//! ```ignore
//! let loader = Arc::new(MyBundleLoader::default());
//!
//! let mut message_source = ResourceBundleMessageSource::default();
//! message_source.set_basenames(["messages"]);
//! message_source.set_bundle_loader(loader);
//!
//! let message = message_source.message("greeting", None, None);
//! ```

use std::collections::HashMap;
use std::fmt;
use std::io;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tracing::warn;

use crate::support::BaseResourceBasedMessageSource;
use crate::util::{MessageFormat, Properties};
use crate::{Locale, MessageSource, MessageSourceResolvable, NoSuchMessageError};

/// The directory, relative to the root of the [`BundleLoader`], that holds the
/// message bundles.
///
/// The constant mirrors `MESSAGES` in `next-web-core`: the basename `messages`
/// is resolved to `messages/messages.properties`, and its variants to
/// `messages/messages_zh_CN.properties`.
const BUNDLE_DIRECTORY: &str = "messages/";

/// The extension of a property based message bundle.
const PROPERTIES_EXTENSION: &str = "properties";

/// Reads the content of the resource bundles a
/// [`ResourceBundleMessageSource`] resolves its messages from.
///
/// The trait plays the role the class loader plays for
/// `java.util.ResourceBundle`: the message source asks for the candidates of a
/// basename and the loader answers with the content of the ones that exist.
pub trait BundleLoader: Send + Sync {
    /// Reads the content of the resource at the given location.
    ///
    /// # Arguments
    ///
    /// * `location` - The location of the resource, relative to the root of the
    ///   loader, for example `messages/messages_zh_CN.properties`.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] when the resource exists but cannot be read. A
    /// location that does not exist is not an error: it has to be reported as
    /// `Ok(None)` so that the message source can continue with the next
    /// candidate.
    fn load(&self, location: &str) -> io::Result<Option<Vec<u8>>>;
}

/// The messages of one basename for one locale.
///
/// A bundle holds the entries of every file that contributed to it, from the
/// locale independent one to the most specific one, together with the compiled
/// patterns of the messages that were rendered so far.
#[derive(Debug)]
pub struct ResourceBundle {
    basename: String,
    locale: Locale,
    properties: Properties,
    message_formats: RwLock<HashMap<String, MessageFormat>>,
}

impl ResourceBundle {
    /// Creates a bundle from the entries it holds.
    fn new(basename: String, locale: Locale, properties: Properties) -> Self {
        Self {
            basename,
            locale,
            properties,
            // Patterns are compiled lazily: a lookup that does not have to
            // render a message must not pay for the parsing of its pattern.
            message_formats: RwLock::new(HashMap::new()),
        }
    }

    /// Returns the basename this bundle was resolved for.
    pub fn basename(&self) -> &str {
        &self.basename
    }

    /// Returns the locale this bundle was resolved for.
    pub fn locale(&self) -> &Locale {
        &self.locale
    }

    /// Returns the entries this bundle holds.
    pub fn properties(&self) -> &Properties {
        &self.properties
    }

    /// Returns the value of the given key, or `None` when the bundle does not
    /// define it.
    ///
    /// Equivalent to `getStringOrNull(ResourceBundle, String)`.
    ///
    /// # Arguments
    ///
    /// * `key` - The message code to look up.
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.properties.get_property(key)
    }

    /// Returns whether the bundle defines the given key.
    ///
    /// Equivalent to `ResourceBundle.containsKey(String)`.
    ///
    /// # Arguments
    ///
    /// * `key` - The message code to look up.
    pub fn contains_key(&self, key: &str) -> bool {
        self.properties.contains_key(key)
    }

    /// Renders the message the bundle defines for the given key.
    ///
    /// Returns `None` when the bundle does not define the key. The compiled
    /// pattern of a key is cached, so rendering the same message repeatedly
    /// parses its pattern only once.
    ///
    /// # Arguments
    ///
    /// * `code` - The message code to look up.
    /// * `args` - The arguments the message is rendered with.
    fn format(&self, code: &str, args: &[&dyn fmt::Display]) -> Option<String> {
        {
            let message_formats = lock(&self.message_formats);
            if let Some(message_format) = message_formats.get(code) {
                return Some(message_format.format(args));
            }
        }

        let pattern = self.properties.get_property(code)?;
        let message_format = MessageFormat::new(pattern);

        let mut message_formats = self
            .message_formats
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let message_format = message_formats
            .entry(code.to_owned())
            .or_insert(message_format);

        Some(message_format.format(args))
    }
}

/// A bundle together with the moment it was loaded.
#[derive(Debug, Clone)]
struct CachedBundle {
    bundle: Arc<ResourceBundle>,
    loaded_at_millis: u64,
}

/// A [`MessageSource`] that resolves its messages in property bundles.
///
/// The type is the Rust counterpart of
/// `org.springframework.context.support.ResourceBundleMessageSource`: it caches
/// the bundles it loaded as well as the compiled pattern of every message it
/// rendered, and it can fall back to the locale of the system, to a configured
/// default locale and to a set of locale independent common messages.
pub struct ResourceBundleMessageSource {
    base: BaseResourceBasedMessageSource,
    common_messages: Option<Arc<Properties>>,
    parent_message_source: Option<Arc<dyn MessageSource>>,
    bundle_loader: Option<Arc<dyn BundleLoader>>,
    always_use_message_format: bool,
    use_code_as_default_message: bool,
    // The loaded bundles, keyed by basename and locale. The cache is shared by
    // the clones of a message source, so that a clone does not have to load the
    // bundles again.
    cached_resource_bundles: Arc<RwLock<HashMap<String, HashMap<Locale, CachedBundle>>>>,
}

impl ResourceBundleMessageSource {
    /// Returns the configuration shared with the other resource based message
    /// sources.
    ///
    /// The configuration is the Rust counterpart of the superclass of the
    /// original, `AbstractResourceBasedMessageSource`.
    pub fn base(&self) -> &BaseResourceBasedMessageSource {
        &self.base
    }

    /// Returns the configuration shared with the other resource based message
    /// sources, for modification.
    pub fn base_mut(&mut self) -> &mut BaseResourceBasedMessageSource {
        &mut self.base
    }

    /// Returns the basenames the messages are searched in, in order.
    ///
    /// Equivalent to `getBasenameSet()`.
    pub fn basenames(&self) -> &[String] {
        self.base.basenames()
    }

    /// Returns the basenames the messages are searched in, in order.
    ///
    /// Equivalent to `getBasenameSet()`.
    pub fn get_basename_set(&self) -> &[String] {
        self.base.basenames()
    }

    /// Replaces the basenames the messages are searched in.
    ///
    /// Equivalent to `setBasenames(String...)`. The loaded bundles are dropped,
    /// since they were resolved for the previous basenames.
    ///
    /// # Arguments
    ///
    /// * `basenames` - The basenames to search, for example
    ///   `["messages", "errors"]`.
    pub fn set_basenames<I>(&mut self, basenames: I)
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        self.base.set_basenames(basenames);
        self.clear_cache();
    }

    /// Adds a basename the messages are searched in.
    ///
    /// Equivalent to `addBasename(String)`.
    ///
    /// # Arguments
    ///
    /// * `basename` - The basename to add.
    pub fn add_basename(&mut self, basename: impl Into<String>) {
        let basename = basename.into();
        if basename.trim().is_empty() || self.base.basenames().contains(&basename) {
            return;
        }

        self.base.add_basename(basename);
        self.clear_cache();
    }

    /// Returns the locale a lookup falls back to when no locale is requested.
    ///
    /// Equivalent to `getDefaultLocale()`.
    pub fn default_locale(&self) -> Option<&Locale> {
        self.base.default_locale()
    }

    /// Replaces the locale a lookup falls back to.
    ///
    /// Equivalent to `setDefaultLocale(Locale)`.
    ///
    /// # Arguments
    ///
    /// * `locale` - The fallback locale, or `None` for the locale of the
    ///   system.
    pub fn set_default_locale(&mut self, locale: Option<Locale>) {
        self.base.set_default_locale(locale);
        self.clear_cache();
    }

    /// Returns whether messages fall back to the locale of the system.
    ///
    /// Equivalent to `isFallbackToSystemLocale()`.
    pub fn fallback_to_system_locale(&self) -> bool {
        self.base.fallback_to_system_locale()
    }

    /// Sets whether messages fall back to the locale of the system.
    ///
    /// Equivalent to `setFallbackToSystemLocale(boolean)`.
    ///
    /// # Arguments
    ///
    /// * `fallback` - Whether the locale of the system is used as a fallback.
    pub fn set_fallback_to_system_locale(&mut self, fallback: bool) {
        self.base.set_fallback_to_system_locale(fallback);
        self.clear_cache();
    }

    /// Returns how long a loaded bundle is cached, or `None` when bundles are
    /// cached forever.
    ///
    /// Equivalent to `getCacheMillis()`.
    pub fn cache_duration(&self) -> Option<Duration> {
        self.base.cache_duration()
    }

    /// Sets how long a loaded bundle is cached.
    ///
    /// Equivalent to `setCacheMillis(long)`.
    ///
    /// # Arguments
    ///
    /// * `duration` - The lifetime of a cached bundle, or `None` to cache the
    ///   bundles forever.
    pub fn set_cache_duration(&mut self, duration: Option<Duration>) {
        self.base.set_cache_duration(duration);
        self.clear_cache();
    }

    /// Returns whether every message is rendered as a [`MessageFormat`], and not
    /// only the messages that declare arguments.
    ///
    /// Equivalent to `isAlwaysUseMessageFormat()`.
    pub fn is_always_use_message_format(&self) -> bool {
        self.always_use_message_format
    }

    /// Sets whether every message is rendered as a [`MessageFormat`].
    ///
    /// Equivalent to `setAlwaysUseMessageFormat(boolean)`.
    ///
    /// # Arguments
    ///
    /// * `always_use_message_format` - Whether the pattern of a message is
    ///   always applied, even when the message declares no argument.
    pub fn set_always_use_message_format(&mut self, always_use_message_format: bool) {
        self.always_use_message_format = always_use_message_format;
    }

    /// Returns whether the code of a message is used as the message when the
    /// lookup failed.
    ///
    /// Equivalent to `isUseCodeAsDefaultMessage()`.
    pub fn is_use_code_as_default_message(&self) -> bool {
        self.use_code_as_default_message
    }

    /// Sets whether the code of a message is used as the message when the lookup
    /// failed.
    ///
    /// Equivalent to `setUseCodeAsDefaultMessage(boolean)`. This is useful
    /// during development only.
    ///
    /// # Arguments
    ///
    /// * `use_code_as_default_message` - Whether a failed lookup renders the
    ///   code instead of failing.
    pub fn set_use_code_as_default_message(&mut self, use_code_as_default_message: bool) {
        self.use_code_as_default_message = use_code_as_default_message;
    }

    /// Returns the common messages, the locale independent messages that are
    /// used when no bundle defines a code.
    ///
    /// Equivalent to `getCommonMessages()`.
    pub fn common_messages(&self) -> Option<&Properties> {
        self.common_messages.as_deref()
    }

    /// Replaces the common messages.
    ///
    /// Equivalent to `setCommonMessages(Properties)`.
    ///
    /// # Arguments
    ///
    /// * `common_messages` - The locale independent messages.
    pub fn set_common_messages(&mut self, common_messages: Arc<Properties>) {
        self.common_messages = Some(common_messages);
    }

    /// Returns the message source a failed lookup is delegated to.
    ///
    /// Equivalent to `getParentMessageSource()`.
    pub fn parent_message_source(&self) -> Option<&Arc<dyn MessageSource>> {
        self.parent_message_source.as_ref()
    }

    /// Sets the message source a failed lookup is delegated to.
    ///
    /// Equivalent to `setParentMessageSource(MessageSource)`.
    ///
    /// # Arguments
    ///
    /// * `parent_message_source` - The message source the lookup falls back to.
    pub fn set_parent_message_source(&mut self, parent_message_source: Arc<dyn MessageSource>) {
        self.parent_message_source = Some(parent_message_source);
    }

    /// Returns the loader the bundles are read with.
    pub fn bundle_loader(&self) -> Option<&Arc<dyn BundleLoader>> {
        self.bundle_loader.as_ref()
    }

    /// Sets the loader the bundles are read with.
    ///
    /// A message source without a loader reports every lookup as missing, so a
    /// message source that is expected to resolve messages has to be given one.
    ///
    /// # Arguments
    ///
    /// * `bundle_loader` - The loader of the bundles.
    pub fn set_bundle_loader(&mut self, bundle_loader: Arc<dyn BundleLoader>) {
        self.bundle_loader = Some(bundle_loader);
        self.clear_cache();
    }

    /// Drops every cached bundle.
    ///
    /// The bundles are loaded again on the next lookup.
    pub fn clear_cache(&self) {
        lock_mut(&self.cached_resource_bundles).clear();
    }

    /// Resolves the given code in the registered bundles, without rendering it.
    ///
    /// Equivalent to `resolveCodeWithoutArguments(String, Locale)`.
    ///
    /// # Arguments
    ///
    /// * `code` - The message code to look up.
    /// * `locale` - The locale to resolve the code for.
    pub fn resolve_code_without_arguments(&self, code: &str, locale: &Locale) -> Option<String> {
        for basename in self.base.basenames() {
            let Some(bundle) = self.get_resource_bundle(basename, locale) else {
                continue;
            };

            if let Some(message) = bundle.get_string(code) {
                return Some(message.to_owned());
            }
        }

        None
    }

    /// Resolves the given code in the registered bundles and renders it.
    ///
    /// Equivalent to `resolveCode(String, Locale)`.
    ///
    /// # Arguments
    ///
    /// * `code` - The message code to look up.
    /// * `locale` - The locale to resolve the code for.
    /// * `args` - The arguments the message is rendered with.
    pub fn resolve_code(
        &self,
        code: &str,
        locale: &Locale,
        args: &[&dyn fmt::Display],
    ) -> Option<String> {
        for basename in self.base.basenames() {
            let Some(bundle) = self.get_resource_bundle(basename, locale) else {
                continue;
            };

            if let Some(message) = bundle.format(code, args) {
                return Some(message);
            }
        }

        None
    }

    /// Returns the bundle of the given basename and locale, loading it when it
    /// is not cached yet.
    ///
    /// Equivalent to `getResourceBundle(String, Locale)`. A bundle that cannot
    /// be found is reported as a warning and resolves to `None`, so that the
    /// caller can continue with the next basename or its parent message source.
    ///
    /// # Arguments
    ///
    /// * `basename` - The basename of the bundle.
    /// * `locale` - The locale of the bundle.
    pub fn get_resource_bundle(
        &self,
        basename: &str,
        locale: &Locale,
    ) -> Option<Arc<ResourceBundle>> {
        if let Some(bundle) = self.get_cached_bundle(basename, locale) {
            return Some(bundle);
        }

        let properties = match self.load_bundle_hierarchy(basename, locale) {
            Ok(properties) => properties,
            Err(error) => {
                warn!("ResourceBundle [{basename}] not found for locale [{locale}]: {error}");
                return None;
            }
        };

        let loaded_at_millis = current_time_millis();
        let bundle = Arc::new(ResourceBundle::new(
            basename.to_owned(),
            locale.clone(),
            properties,
        ));

        // Another thread may have loaded the same bundle in the meantime, in
        // which case the bundle that is already cached is the one to use.
        if let Some(cached) = self.get_cached_bundle(basename, locale) {
            return Some(cached);
        }

        lock_mut(&self.cached_resource_bundles)
            .entry(basename.to_owned())
            .or_default()
            .insert(
                locale.clone(),
                CachedBundle {
                    bundle: Arc::clone(&bundle),
                    loaded_at_millis,
                },
            );

        Some(bundle)
    }

    /// Returns the cached bundle of the given basename and locale, or `None`
    /// when it is not cached or is no longer valid.
    ///
    /// # Arguments
    ///
    /// * `basename` - The basename of the bundle.
    /// * `locale` - The locale of the bundle.
    fn get_cached_bundle(&self, basename: &str, locale: &Locale) -> Option<Arc<ResourceBundle>> {
        let cache = lock(&self.cached_resource_bundles);
        let cached = cache.get(basename)?.get(locale)?;

        if let Some(duration) = self.base.cache_duration() {
            let lifetime = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
            if cached.loaded_at_millis.saturating_add(lifetime) <= current_time_millis() {
                return None;
            }
        }

        Some(Arc::clone(&cached.bundle))
    }

    /// Loads the bundle of the given basename and locale.
    ///
    /// `loadBundle(Reader)` contributes a single candidate; the bundle of a
    /// lookup merges every candidate file that exists, from the least to the
    /// most specific one, so that a locale specific entry overrides the value of
    /// the locale independent one.
    ///
    /// # Arguments
    ///
    /// * `basename` - The basename of the bundle.
    /// * `locale` - The locale of the bundle.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] when none of the candidates exists or when one
    /// of them cannot be read.
    fn load_bundle_hierarchy(&self, basename: &str, locale: &Locale) -> io::Result<Properties> {
        let mut merged = Properties::new();
        let mut found = false;

        for filename in self
            .calculate_bundle_filenames(basename, locale)
            .into_iter()
            .rev()
        {
            let location = to_resource_name(&filename, PROPERTIES_EXTENSION);
            if let Some(properties) = self.load_bundle_file(&location)? {
                merged.put_all(&properties);
                found = true;
            }
        }

        if !found {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("No bundle found for basename {basename} and locale {locale}"),
            ));
        }

        Ok(merged)
    }

    /// Loads a single bundle file, or returns `None` when it does not exist.
    ///
    /// # Arguments
    ///
    /// * `location` - The location of the bundle file.
    fn load_bundle_file(&self, location: &str) -> io::Result<Option<Properties>> {
        let Some(bundle_loader) = self.bundle_loader.as_deref() else {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "No bundle loader is configured for the message source",
            ));
        };

        let Some(content) = bundle_loader.load(location)? else {
            return Ok(None);
        };

        let mut properties = Properties::new();
        properties.load_from_bytes(&content)?;

        Ok(Some(properties))
    }

    /// Returns the bundle names to look for, from the most to the least specific
    /// one.
    ///
    /// `getFallbackLocale(String, Locale)` contributes the locale the lookup
    /// falls back to, and the basename itself is always the last candidate.
    ///
    /// # Arguments
    ///
    /// * `basename` - The basename of the bundle.
    /// * `locale` - The locale of the bundle.
    fn calculate_bundle_filenames(&self, basename: &str, locale: &Locale) -> Vec<String> {
        let mut filenames = self.calculate_locale_filenames(basename, locale);
        let fallback = self.calculate_fallback_locale(locale);

        if let Some(fallback) = fallback {
            for filename in self.calculate_locale_filenames(basename, &fallback) {
                if !filenames.contains(&filename) {
                    filenames.push(filename);
                }
            }
        }

        if !filenames.iter().any(|filename| filename == basename) {
            filenames.push(basename.to_owned());
        }

        filenames
    }

    /// Returns the locale the lookup falls back to.
    ///
    /// Equivalent to `getFallbackLocale(String, Locale)`: the configured default
    /// locale when there is one, the locale of the system otherwise, and no
    /// fallback at all when `fallback_to_system_locale` is turned off.
    ///
    /// # Arguments
    ///
    /// * `locale` - The locale the caller asked for.
    fn calculate_fallback_locale(&self, locale: &Locale) -> Option<Locale> {
        if let Some(default_locale) = self.base.default_locale() {
            return (default_locale != locale).then(|| default_locale.clone());
        }

        if !self.base.fallback_to_system_locale() {
            return None;
        }

        let system_locale = Locale::system_locale();
        (system_locale != *locale).then_some(system_locale)
    }

    /// Returns the names of the bundles of one locale, from the most to the
    /// least specific one, without the basename itself.
    ///
    /// # Arguments
    ///
    /// * `basename` - The basename of the bundle.
    /// * `locale` - The locale of the bundle.
    fn calculate_locale_filenames(&self, basename: &str, locale: &Locale) -> Vec<String> {
        let mut parts = Vec::with_capacity(3);
        if !locale.language().is_empty() {
            parts.push(locale.language().to_owned());
        }
        if let Some(country) = locale.country() {
            parts.push(country.to_owned());
        }
        if let Some(variant) = locale.variant() {
            parts.push(variant.to_owned());
        }

        let mut filenames = Vec::with_capacity(parts.len());
        while !parts.is_empty() {
            filenames.push(format!("{basename}_{}", parts.join("_")));
            parts.pop();
        }

        filenames
    }

    /// Resolves the locale a lookup is performed with.
    ///
    /// The locale of the caller wins, then the default locale of the message
    /// source, then the locale of the system.
    ///
    /// # Arguments
    ///
    /// * `locale` - The locale the caller asked for.
    fn resolved_locale(&self, locale: Option<&Locale>) -> Locale {
        locale
            .cloned()
            .or_else(|| self.base.default_locale().cloned())
            .unwrap_or_else(Locale::system_locale)
    }

    /// Resolves a message, without applying any default message.
    ///
    /// Equivalent to `getMessageInternal(String, Object[], Locale)`: the message
    /// is looked up in the bundles, then in the common messages, and finally in
    /// the parent message source.
    ///
    /// # Arguments
    ///
    /// * `code` - The message code to look up.
    /// * `args` - The arguments the message is rendered with.
    /// * `locale` - The locale the caller asked for.
    fn get_message_internal(
        &self,
        code: &str,
        args: &[&dyn fmt::Display],
        locale: Option<&Locale>,
    ) -> Option<String> {
        let locale = self.resolved_locale(locale);

        if !self.always_use_message_format && args.is_empty() {
            // Optimized resolution: no argument has to be applied, so no
            // pattern has to be compiled either.
            if let Some(message) = self.resolve_code_without_arguments(code, &locale) {
                return Some(message);
            }
        } else if let Some(message) = self.resolve_code(code, &locale, args) {
            return Some(message);
        }

        if let Some(common_message) = self
            .common_messages
            .as_deref()
            .and_then(|common_messages| common_messages.get_property(code))
        {
            return Some(common_message.to_owned());
        }

        if let Some(parent_message_source) = self.parent_message_source.as_deref() {
            return parent_message_source.message_or_default(code, args, None, Some(&locale));
        }

        None
    }

    /// Renders a default message with the given arguments.
    ///
    /// Equivalent to `renderDefaultMessage(String, Object[], Locale)`. The
    /// pattern of the default message is only applied when it has to be, which
    /// is what the JDK does through `formatMessage`.
    ///
    /// # Arguments
    ///
    /// * `default_message` - The default message to render.
    /// * `args` - The arguments the default message is rendered with.
    fn render_default_message(&self, default_message: &str, args: &[&dyn fmt::Display]) -> String {
        if !self.always_use_message_format && args.is_empty() {
            return default_message.to_owned();
        }

        MessageFormat::new(default_message).format(args)
    }

    /// Returns the message a failed lookup falls back to.
    ///
    /// Equivalent to `getDefaultMessage(String)`: the code of the message is
    /// used when `useCodeAsDefaultMessage` is turned on.
    ///
    /// # Arguments
    ///
    /// * `code` - The message code that could not be resolved.
    fn get_default_message(&self, code: &str) -> Option<String> {
        self.use_code_as_default_message.then(|| code.to_owned())
    }

    /// Returns the default message of the given resolvable.
    ///
    /// Equivalent to `getDefaultMessage(MessageSourceResolvable, Locale)`.
    ///
    /// # Arguments
    ///
    /// * `resolvable` - The value object the default message is read from.
    /// * `codes` - The codes of the resolvable.
    /// * `args` - The arguments of the resolvable.
    fn get_resolvable_default_message(
        &self,
        resolvable: &dyn MessageSourceResolvable,
        codes: &[String],
        args: &[&dyn fmt::Display],
    ) -> Option<String> {
        let Some(default_message) = resolvable.default_message() else {
            return codes.last().and_then(|code| self.get_default_message(code));
        };

        if let Some(code) = codes.last() {
            // A default message that is identical to the last code enforces
            // `useCodeAsDefaultMessage` for this particular message.
            if code == default_message {
                return Some(
                    self.get_default_message(code)
                        .unwrap_or_else(|| default_message.to_owned()),
                );
            }
        }

        Some(self.render_default_message(default_message, args))
    }
}

impl MessageSource for ResourceBundleMessageSource {
    fn message_or_default(
        &self,
        code: &str,
        args: &[&dyn fmt::Display],
        default_message: Option<&str>,
        locale: Option<&Locale>,
    ) -> Option<String> {
        if let Some(message) = self.get_message_internal(code, args, locale) {
            return Some(message);
        }

        if let Some(default_message) = default_message {
            return Some(self.render_default_message(default_message, args));
        }

        self.get_default_message(code)
    }

    fn message(
        &self,
        code: &str,
        args: &[&dyn fmt::Display],
        locale: Option<&Locale>,
    ) -> Result<String, NoSuchMessageError> {
        if let Some(message) = self.get_message_internal(code, args, locale) {
            return Ok(message);
        }

        if let Some(fallback) = self.get_default_message(code) {
            return Ok(fallback);
        }

        Err(NoSuchMessageError::new(code, locale))
    }

    fn message_from_resolvable(
        &self,
        resolvable: &dyn MessageSourceResolvable,
        locale: Option<&Locale>,
    ) -> Result<String, NoSuchMessageError> {
        let codes = resolvable.codes().unwrap_or_default();
        if codes.is_empty() && resolvable.default_message().is_none() {
            return Err(NoSuchMessageError::new("", locale));
        }

        let owned = resolvable
            .arguments()
            .map(|arguments| arguments.iter().map(AsRef::as_ref).collect::<Vec<_>>())
            .unwrap_or_default();
        let args = owned.as_slice();

        for code in codes {
            if let Some(message) = self.get_message_internal(code, args, locale) {
                return Ok(message);
            }
        }

        if let Some(message) = self.get_resolvable_default_message(resolvable, codes, args) {
            return Ok(message);
        }

        let code = codes.last().map(String::as_str).unwrap_or_default();
        Err(NoSuchMessageError::new(code, locale))
    }
}

impl fmt::Debug for ResourceBundleMessageSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceBundleMessageSource")
            .field("base", &self.base)
            .field(
                "common_messages",
                &self.common_messages.as_ref().map(|messages| messages.len()),
            )
            .field(
                "parent_message_source",
                &self.parent_message_source.is_some(),
            )
            .field("bundle_loader", &self.bundle_loader.is_some())
            .field("always_use_message_format", &self.always_use_message_format)
            .field(
                "use_code_as_default_message",
                &self.use_code_as_default_message,
            )
            .finish()
    }
}

impl fmt::Display for ResourceBundleMessageSource {
    /// Writes the basenames of this message source, the way `toString()` does.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ResourceBundleMessageSource: basenames={:?}",
            self.base.basenames()
        )
    }
}

impl Clone for ResourceBundleMessageSource {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            common_messages: self.common_messages.clone(),
            parent_message_source: self.parent_message_source.clone(),
            bundle_loader: self.bundle_loader.clone(),
            always_use_message_format: self.always_use_message_format,
            use_code_as_default_message: self.use_code_as_default_message,
            cached_resource_bundles: Arc::clone(&self.cached_resource_bundles),
        }
    }
}

impl Default for ResourceBundleMessageSource {
    fn default() -> Self {
        Self {
            base: BaseResourceBasedMessageSource::default(),
            common_messages: None,
            parent_message_source: None,
            bundle_loader: None,
            always_use_message_format: false,
            use_code_as_default_message: false,
            cached_resource_bundles: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// Returns the reader of a cache, ignoring a poisoned lock.
///
/// A panic that happened while a cache was written does not leave it in an
/// inconsistent state, so the value it holds is still used.
fn lock<T>(cache: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    cache
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Returns the writer of a cache, ignoring a poisoned lock.
///
/// A panic that happened while a cache was written does not leave it in an
/// inconsistent state, so the value it holds is still used.
fn lock_mut<T>(cache: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    cache
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Returns the location of a bundle, relative to the root of the loader.
///
/// Equivalent to `toResourceName(String, String)`: the dots of the bundle name
/// are the package separators of the JDK, so `test.theme` is read from
/// `messages/test/theme.properties`.
///
/// # Arguments
///
/// * `bundle_name` - The name of the bundle, without its extension.
/// * `extension` - The extension of the bundle file.
fn to_resource_name(bundle_name: &str, extension: &str) -> String {
    let path = bundle_name.replace('.', "/");
    format!("{BUNDLE_DIRECTORY}{path}.{extension}")
}

/// Returns the current time, in milliseconds since the Unix epoch.
fn current_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    /// A loader that serves the bundles it is given and counts how often it was
    /// asked for one.
    #[derive(Default)]
    struct InMemoryBundleLoader {
        bundles: HashMap<String, Vec<u8>>,
        loads: AtomicUsize,
    }

    impl InMemoryBundleLoader {
        fn new(entries: impl IntoIterator<Item = (&'static str, &'static str)>) -> Arc<Self> {
            Arc::new(Self {
                bundles: entries
                    .into_iter()
                    .map(|(location, content)| (location.to_string(), content.as_bytes().to_vec()))
                    .collect(),
                loads: AtomicUsize::new(0),
            })
        }

        fn loads(&self) -> usize {
            self.loads.load(Ordering::SeqCst)
        }
    }

    impl BundleLoader for InMemoryBundleLoader {
        fn load(&self, location: &str) -> io::Result<Option<Vec<u8>>> {
            self.loads.fetch_add(1, Ordering::SeqCst);
            Ok(self.bundles.get(location).cloned())
        }
    }

    fn message_source(
        loader: Arc<InMemoryBundleLoader>,
        basenames: impl IntoIterator<Item = &'static str>,
    ) -> ResourceBundleMessageSource {
        let mut message_source = ResourceBundleMessageSource::default();
        message_source.set_basenames(basenames);
        message_source.set_fallback_to_system_locale(false);
        message_source.set_bundle_loader(loader);
        message_source
    }

    #[test]
    fn resolves_a_message_of_the_locale_independent_bundle() {
        let loader =
            InMemoryBundleLoader::new([("messages/messages.properties", "hello=Hello, world!\n")]);
        let message_source = message_source(loader, ["messages"]);

        assert_eq!(
            message_source
                .message("hello", &[], Some(&Locale::new("zh", "CN")))
                .unwrap(),
            "Hello, world!"
        );
    }

    #[test]
    fn a_locale_specific_entry_overrides_the_locale_independent_one() {
        let loader = InMemoryBundleLoader::new([
            (
                "messages/messages.properties",
                "hello=Hello!\ngreeting=Hi!\n",
            ),
            ("messages/messages_zh.properties", "hello=Ni hao!\n"),
            (
                "messages/messages_zh_CN.properties",
                "hello=Ni hao, shijie!\n",
            ),
        ]);
        let message_source = message_source(loader, ["messages"]);
        let zh_cn = Locale::new("zh", "CN");
        let zh = Locale::for_language("zh");

        assert_eq!(
            message_source.message("hello", &[], Some(&zh_cn)).unwrap(),
            "Ni hao, shijie!"
        );
        assert_eq!(
            message_source.message("hello", &[], Some(&zh)).unwrap(),
            "Ni hao!"
        );
        // An entry the locale specific bundle does not define falls back to the
        // locale independent bundle.
        assert_eq!(
            message_source
                .message("greeting", &[], Some(&zh_cn))
                .unwrap(),
            "Hi!"
        );
        // A locale without a bundle of its own falls back to the base bundle.
        assert_eq!(
            message_source
                .message("hello", &[], Some(&Locale::english()))
                .unwrap(),
            "Hello!"
        );
    }

    #[test]
    fn renders_the_arguments_of_a_message() {
        let loader = InMemoryBundleLoader::new([(
            "messages/messages.properties",
            "name=Your name is {2}, {1}, {0}.\n",
        )]);
        let message_source = message_source(loader, ["messages"]);

        assert_eq!(
            message_source
                .message("name", &[&"a", &"b", &"c"], None)
                .unwrap(),
            "Your name is c, b, a."
        );
    }

    #[test]
    fn fails_when_no_bundle_defines_the_code() {
        let loader =
            InMemoryBundleLoader::new([("messages/messages.properties", "hello=Hello!\n")]);
        let message_source = message_source(loader, ["messages"]);

        assert_eq!(
            message_source
                .message("missing", &[], None)
                .unwrap_err()
                .code,
            "missing"
        );
        assert_eq!(
            message_source.message_or_default("missing", &[], None, None),
            None
        );
        assert_eq!(
            message_source.message_or_default("missing", &[], Some("Fallback"), None),
            Some("Fallback".to_string())
        );
        assert_eq!(
            message_source.message_or_default("missing", &[], Some("Bye {0}"), None),
            Some("Bye {0}".to_string())
        );
    }

    #[test]
    fn uses_the_code_as_the_default_message_when_configured() {
        let loader =
            InMemoryBundleLoader::new([("messages/messages.properties", "hello=Hello!\n")]);
        let mut message_source = message_source(loader, ["messages"]);
        message_source.set_use_code_as_default_message(true);

        assert_eq!(
            message_source.message("missing", &[], None).unwrap(),
            "missing"
        );
    }

    #[test]
    fn resolves_common_messages_before_failing() {
        let loader =
            InMemoryBundleLoader::new([("messages/messages.properties", "hello=Hello!\n")]);
        let mut message_source = message_source(loader, ["messages"]);
        message_source.set_common_messages(Arc::new(Properties::from_iter([(
            "common.hello",
            "Hello from the common messages!",
        )])));

        assert_eq!(
            message_source.message("common.hello", &[], None).unwrap(),
            "Hello from the common messages!"
        );
    }

    #[test]
    fn caches_the_bundles_forever_by_default() {
        let loader =
            InMemoryBundleLoader::new([("messages/messages.properties", "hello=Hello!\n")]);
        let message_source = message_source(Arc::clone(&loader), ["messages"]);

        assert!(message_source.message("hello", &[], None).is_ok());
        let loads_after_first_lookup = loader.loads();
        assert!(message_source.message("hello", &[], None).is_ok());
        assert_eq!(loader.loads(), loads_after_first_lookup);
    }

    #[test]
    fn reloads_the_bundles_when_the_cache_duration_is_zero() {
        let loader =
            InMemoryBundleLoader::new([("messages/messages.properties", "hello=Hello!\n")]);
        let mut message_source = message_source(Arc::clone(&loader), ["messages"]);
        message_source.set_cache_duration(Some(Duration::ZERO));

        assert!(message_source.message("hello", &[], None).is_ok());
        let loads_after_first_lookup = loader.loads();
        assert!(message_source.message("hello", &[], None).is_ok());
        assert!(loader.loads() > loads_after_first_lookup);
    }

    #[test]
    fn reports_a_missing_bundle_as_a_missing_message() {
        let loader = InMemoryBundleLoader::new([]);
        let message_source = message_source(loader, ["messages"]);

        assert!(message_source.message("hello", &[], None).is_err());
        assert_eq!(message_source.basenames(), ["messages"]);
        assert_eq!(
            message_source.to_string(),
            "ResourceBundleMessageSource: basenames=[\"messages\"]"
        );
    }

    #[test]
    fn a_missing_bundle_loader_is_reported_as_a_missing_message() {
        let mut message_source = ResourceBundleMessageSource::default();
        message_source.set_basenames(["messages"]);

        assert!(message_source.message("hello", &[], None).is_err());
    }

    #[test]
    fn reads_the_bundles_as_utf_8() {
        let loader =
            InMemoryBundleLoader::new([("messages/messages.properties", "greeting=caf\u{E9}\n")]);
        let message_source = message_source(loader, ["messages"]);

        assert_eq!(
            message_source.message("greeting", &[], None).unwrap(),
            "caf\u{E9}"
        );
    }

    #[test]
    fn resolves_the_locale_of_the_caller_then_the_default_locale() {
        let loader = InMemoryBundleLoader::new([
            ("messages/messages.properties", "hello=Hello!\n"),
            ("messages/messages_zh.properties", "hello=Ni hao!\n"),
            ("messages/messages_en.properties", "hello=Hi!\n"),
        ]);
        let mut message_source = message_source(loader, ["messages"]);
        message_source.set_default_locale(Some(Locale::for_language("zh")));

        assert_eq!(
            message_source.message("hello", &[], None).unwrap(),
            "Ni hao!"
        );
        assert_eq!(
            message_source
                .message("hello", &[], Some(&Locale::english()))
                .unwrap(),
            "Hi!"
        );
    }

    #[test]
    fn to_resource_name_replaces_the_package_separators() {
        assert_eq!(
            to_resource_name("test.theme", PROPERTIES_EXTENSION),
            "messages/test/theme.properties"
        );
        assert_eq!(
            to_resource_name("messages_zh_CN", PROPERTIES_EXTENSION),
            "messages/messages_zh_CN.properties"
        );
    }

    #[test]
    fn is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync + fmt::Debug>() {}
        assert_send_and_sync::<ResourceBundleMessageSource>();

        let message_source: Arc<dyn MessageSource> =
            Arc::new(ResourceBundleMessageSource::default());
        assert!(message_source.message("missing", &[], None).is_err());
    }
}
