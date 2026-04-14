use next_web_macros::properties;
use rudi_dev::singleton;

#[singleton(default, binds=[Self::into_properties])]
#[properties(prefix = "next.messages")]
#[derive(Debug, Default, Clone, serde::Deserialize)]
pub struct MessageSourceProperties {
    /// Default Locale
    default_local: Option<String>,

    /// Resource bundle basename
    /// messages, errors, validation
    base_name: Option<String>,

    /// Whether to fall back to the system Locale if no files for a specific Locale have been found. if this is
    /// turned off, the only fallback will be the default file (e.g. "messages.properties" for basename "messages").
    fallback_to_system_locale: Option<bool>,

    /// Loaded resource bundle files cache duration. When not set, bundles are cached forever.
    /// Will default to using seconds.
    cache_time: Option<u64>,
}

impl MessageSourceProperties {
    pub fn default_local(&self) -> Option<&str> {
        self.default_local.as_deref()
    }

    pub fn base_name(&self) -> Vec<String> {
        self.base_name
            .as_ref()
            .map(|s| s.trim_end())
            .map(|s| s.split(",").map(str::to_string).collect::<Vec<_>>())
            .unwrap_or(vec!["messages".to_string()])
    }

    pub fn fallback_to_system_locale(&self) -> bool {
        self.fallback_to_system_locale.unwrap_or(true)
    }

    pub fn cache_time(&self) -> u64 {
        self.cache_time.unwrap_or_default()
    }

    pub fn set_default_local<T>(&mut self, local: T)
    where
        T: Into<String>,
    {
        self.default_local = Some(local.into());
    }

    pub fn set_base_name<I>(&mut self, base_name: I)
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        self.base_name = Some(base_name.into_iter().map(Into::into).collect());
    }

    pub fn set_fallback_to_system_locale(&mut self, fallback_to_system_locale: bool) {
        self.fallback_to_system_locale = Some(fallback_to_system_locale);
    }

    pub fn set_cache_time(&mut self, cache_time: u64) {
        self.cache_time = Some(cache_time);
    }
}
