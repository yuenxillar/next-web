//! Port of `org.springframework.context.support.AbstractResourceBasedMessageSource`.

use std::time::Duration;

use crate::Locale;

/// The basename a message source uses when none is configured.
pub const DEFAULT_BASENAME: &str = "messages";

/// The configuration shared by every message source that resolves its messages
/// from resources.
///
/// The type holds the basenames the messages are searched in, the locale the
/// lookup falls back to and how long a loaded bundle may stay cached. The
/// bundles themselves are read as UTF-8, since a Rust string is UTF-8. It is
/// embedded in
/// [`ResourceBundleMessageSource`](crate::support::ResourceBundleMessageSource),
/// which is the type applications use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseResourceBasedMessageSource {
    basenames: Vec<String>,
    default_locale: Option<Locale>,
    fallback_to_system_locale: bool,
    cache_duration: Option<Duration>,
}

impl BaseResourceBasedMessageSource {
    /// Returns the basenames, in the order they are searched.
    ///
    /// Equivalent to `getBasenameSet()`.
    pub fn basenames(&self) -> &[String] {
        &self.basenames
    }

    /// Replaces the basenames with the given ones.
    ///
    /// Equivalent to `setBasenames(String...)`. Blank entries are ignored.
    ///
    /// # Arguments
    ///
    /// * `basenames` - The basenames to search, for example `["messages"]`.
    ///
    /// # Panics
    ///
    /// Panics when no usable basename is left, which is what
    /// `Assert.notEmpty(basenames, ...)` does in the original.
    pub fn set_basenames<I>(&mut self, basenames: I)
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        let basenames = basenames
            .into_iter()
            .map(Into::into)
            .filter(|basename| !basename.trim().is_empty())
            .collect::<Vec<_>>();

        assert!(
            !basenames.is_empty(),
            "At least one non-empty basename is required for message resolution"
        );

        self.basenames = basenames;
    }

    /// Adds a basename when it is not present yet.
    ///
    /// Equivalent to `addBasename(String)`. A blank basename is ignored.
    ///
    /// # Arguments
    ///
    /// * `basename` - The basename to add.
    pub fn add_basename(&mut self, basename: impl Into<String>) {
        let basename = basename.into();
        if basename.trim().is_empty() {
            return;
        }

        if !self.basenames.iter().any(|existing| existing == &basename) {
            self.basenames.push(basename);
        }
    }

    /// Returns the locale a lookup falls back to when no locale is requested.
    ///
    /// Equivalent to `getDefaultLocale()`.
    pub fn default_locale(&self) -> Option<&Locale> {
        self.default_locale.as_ref()
    }

    /// Replaces the locale a lookup falls back to.
    ///
    /// Equivalent to `setDefaultLocale(Locale)`. Passing `None` restores the
    /// behavior of the JDK, which is to use the locale of the system.
    ///
    /// # Arguments
    ///
    /// * `locale` - The fallback locale.
    pub fn set_default_locale(&mut self, locale: Option<Locale>) {
        self.default_locale = locale;
    }

    /// Returns whether messages fall back to the locale of the system when the
    /// bundle of the requested locale does not exist.
    ///
    /// Equivalent to `isFallbackToSystemLocale()`.
    pub fn fallback_to_system_locale(&self) -> bool {
        self.fallback_to_system_locale
    }

    /// Sets whether messages fall back to the locale of the system.
    ///
    /// Equivalent to `setFallbackToSystemLocale(boolean)`. When this is turned
    /// off, the only fallback of a message is the locale independent bundle,
    /// for example `messages.properties` for the basename `messages`.
    ///
    /// # Arguments
    ///
    /// * `fallback` - Whether the locale of the system is used as a fallback.
    pub fn set_fallback_to_system_locale(&mut self, fallback: bool) {
        self.fallback_to_system_locale = fallback;
    }

    /// Returns how long a loaded bundle is cached, or `None` when bundles are
    /// cached forever.
    ///
    /// Equivalent to `getCacheMillis()`, where a negative value means to cache
    /// forever.
    pub fn cache_duration(&self) -> Option<Duration> {
        self.cache_duration
    }

    /// Sets how long a loaded bundle is cached.
    ///
    /// Equivalent to `setCacheMillis(long)`. Passing `None` caches the bundles
    /// forever, which is the default; passing a zero duration reloads them on
    /// every lookup.
    ///
    /// # Arguments
    ///
    /// * `duration` - The lifetime of a cached bundle.
    pub fn set_cache_duration(&mut self, duration: Option<Duration>) {
        self.cache_duration = duration;
    }

    /// Sets how long a loaded bundle is cached, in seconds.
    ///
    /// Equivalent to `setCacheSeconds(int)`. A negative value caches the
    /// bundles forever and a value of zero reloads them on every lookup.
    ///
    /// # Arguments
    ///
    /// * `seconds` - The lifetime of a cached bundle, in seconds.
    pub fn set_cache_seconds(&mut self, seconds: i64) {
        if seconds < 0 {
            self.cache_duration = None;
        } else {
            self.cache_duration = Some(Duration::from_secs(seconds as u64));
        }
    }
}

impl Default for BaseResourceBasedMessageSource {
    fn default() -> Self {
        Self {
            basenames: vec![DEFAULT_BASENAME.to_owned()],
            default_locale: None,
            fallback_to_system_locale: true,
            cache_duration: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_resource_based_message_source() {
        let base = BaseResourceBasedMessageSource::default();

        assert_eq!(base.basenames(), [DEFAULT_BASENAME]);
        assert_eq!(base.default_locale(), None);
        assert!(base.fallback_to_system_locale());
        assert_eq!(base.cache_duration(), None);
    }

    #[test]
    fn set_basenames_ignores_blank_entries() {
        let mut base = BaseResourceBasedMessageSource::default();
        base.set_basenames(["messages", " ", "errors"]);

        assert_eq!(base.basenames(), ["messages", "errors"]);

        base.add_basename("errors");
        base.add_basename("");
        assert_eq!(base.basenames(), ["messages", "errors"]);
    }

    #[test]
    #[should_panic(expected = "At least one non-empty basename")]
    fn set_basenames_requires_a_usable_basename() {
        BaseResourceBasedMessageSource::default().set_basenames([" "]);
    }

    #[test]
    fn cache_seconds_follow_the_jdk_convention() {
        let mut base = BaseResourceBasedMessageSource::default();

        base.set_cache_seconds(-1);
        assert_eq!(base.cache_duration(), None);

        base.set_cache_seconds(30);
        assert_eq!(base.cache_duration(), Some(Duration::from_secs(30)));

        base.set_cache_seconds(0);
        assert_eq!(base.cache_duration(), Some(Duration::ZERO));
    }
}
