use next_web_macros::configuration_properties;

/// Configuration properties for the message source.
///
/// Bound from the `next.messages` prefix. All fields are optional so that
/// defaults can be applied lazily by the accessor methods, matching the
/// behavior of the original configuration.
///
/// The `#[singleton]` attribute registers this type as a singleton and
/// converts it into properties via [`Self::into_properties`].
#[configuration_properties(prefix = "next.messages")]
#[derive(Debug, Default, Clone, serde::Deserialize)]
pub struct MessageSourceProperties {
    /// Resource bundle basenames, comma-separated.
    ///
    /// For example `"messages, errors, validation"`. When unset, the accessor
    /// falls back to `["messages"]`.
    base_name: Option<String>,

    /// Whether to fall back to the system locale if no files for a specific
    /// locale have been found.
    ///
    /// If this is turned off, the only fallback will be the default file
    /// (e.g. `"messages.properties"` for basename `"messages"`). Defaults to
    /// `true` when unset.
    fallback_to_system_locale: Option<bool>,

    /// Cache duration for loaded resource bundle files, in seconds.
    ///
    /// When not set, bundles are cached forever. Defaults to using seconds.
    cache_duration: Option<u64>,

    /// Whether to use the message code as the default message instead of
    /// throwing a "NoSuchMessageError".
    ///
    /// Recommended during development only. Defaults to `false` when unset.
    use_code_as_default_message: Option<bool>,

    /// List of locale-independent property file resources containing common
    /// messages.
    common_messages: Option<Vec<String>>,

    /// Whether to always apply the `MessageFormat` rules, parsing even messages
    /// without arguments.
    ///
    /// Defaults to `false` when unset.
    always_use_message_format: Option<bool>,
}

impl MessageSourceProperties {
    /// Returns the resource bundle basenames as a list.
    ///
    /// The stored value is comma-separated; surrounding whitespace on the whole
    /// string is trimmed before splitting. When unset, returns `["messages"]`.
    pub fn base_name(&self) -> Vec<String> {
        self.base_name
            .as_ref()
            .map(|s| s.trim_end())
            .map(|s| s.split(",").map(str::to_string).collect::<Vec<_>>())
            .unwrap_or(vec!["messages".to_owned()])
    }

    /// Returns whether to fall back to the system locale.
    ///
    /// Defaults to `true` when unset.
    pub fn fallback_to_system_locale(&self) -> bool {
        self.fallback_to_system_locale.unwrap_or(true)
    }

    /// Returns how long the loaded bundles are cached, or `None` when they are
    /// cached forever.
    pub fn cache_duration(&self) -> Option<u64> {
        self.cache_duration
    }

    /// Returns the locale-independent resources containing common messages,
    /// if any.
    pub fn common_messages(&self) -> Option<&[String]> {
        self.common_messages.as_deref()
    }

    /// Returns whether to use the message code as the default message.
    ///
    /// Defaults to `false` when unset.
    pub fn is_use_code_as_default_message(&self) -> bool {
        self.use_code_as_default_message.unwrap_or(false)
    }

    /// Returns whether to always apply the `MessageFormat` rules.
    ///
    /// Defaults to `false` when unset.
    pub fn is_always_use_message_format(&self) -> bool {
        self.always_use_message_format.unwrap_or(false)
    }

    /// Sets the resource bundle basenames.
    ///
    /// Each item is converted into a `String` and stored as a comma-separated
    /// list by the underlying binding layer.
    pub fn set_base_name<I>(&mut self, base_name: I)
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        self.base_name = Some(base_name.into_iter().map(Into::into).collect());
    }

    /// Sets whether to fall back to the system locale.
    pub fn set_fallback_to_system_locale(&mut self, fallback_to_system_locale: bool) {
        self.fallback_to_system_locale = Some(fallback_to_system_locale);
    }

    /// Sets the cache duration for loaded bundles, in seconds.
    pub fn set_cache_duration(&mut self, cache_duration: u64) {
        self.cache_duration = Some(cache_duration);
    }

    /// Sets whether to use the message code as the default message.
    pub fn set_use_code_as_default_message(&mut self, use_code_as_default_message: bool) {
        self.use_code_as_default_message = Some(use_code_as_default_message);
    }

    /// Sets the locale-independent resources containing common messages.
    pub fn set_common_messages<I>(&mut self, common_messages: I)
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        self.common_messages = Some(common_messages.into_iter().map(Into::into).collect());
    }

    /// Sets whether to always apply the `MessageFormat` rules.
    pub fn set_always_use_message_format(&mut self, always_use_message_format: bool) {
        self.always_use_message_format = Some(always_use_message_format);
    }
}
