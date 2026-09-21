//! Application properties.

use std::collections::HashSet;

use next_web_core::env::ConfigurableEnvironment;

use crate::banner::BannerMode;

/// Property key controlling whether singleton  overridden.
const ALLOW_OVERRIDE: &str = "next.main.allow_override";

/// Property key controlling how the banner is displayed.
const BANNER_MODE: &str = "next.main.banner_mode";

/// Property key controlling whether startup information is logged.
const LOG_STARTUP_INFO: &str = "next.main.log_startup_info";

/// Property key controlling whether a shutdown hook is registered.
const REGISTER_SHUTDOWN_HOOK: &str = "next.main.register_shutdown_hook";

/// Property key listing the sources to include in the application context.
const SOURCES: &str = "next.main.sources";

/// Next application properties.
#[derive(Debug)]
pub struct ApplicationProperties {
    /// Should it be allowed to overwrite existing providers.
    allow_override: bool,

    /// Mode used to display the banner when the application runs.
    banner_mode: BannerMode,

    /// Whether to log information about the application when it starts.
    log_startup_info: bool,

    /// Whether the application should have a shutdown hook registered.
    register_shutdown_hook: bool,

    /// Sources (type names or XML resource locations) to include in the
    /// ApplicationContext.
    sources: HashSet<String>,
}

#[allow(dead_code)]
impl ApplicationProperties {
    /// Returns whether the context should allow overriding existing providers.
    pub fn is_allow_override(&self) -> bool {
        self.allow_override
    }

    /// Sets whether the context should allow overriding existing providers.
    pub fn set_allow_override(&mut self, allow_override: bool) {
        self.allow_override = allow_override;
    }

    /// Returns the banner mode, resolving the default based on the environment if unset.
    pub fn get_banner_mode(&self) -> BannerMode {
        self.banner_mode
    }

    /// Sets the banner mode.
    pub fn set_banner_mode(&mut self, banner_mode: BannerMode) {
        self.banner_mode = banner_mode;
    }

    /// Returns whether startup information is logged.
    pub fn is_log_startup_info(&self) -> bool {
        self.log_startup_info
    }

    /// Sets whether startup information is logged.
    pub fn set_log_startup_info(&mut self, log_startup_info: bool) {
        self.log_startup_info = log_startup_info;
    }

    /// Returns whether a shutdown hook is registered.
    pub fn is_register_shutdown_hook(&self) -> bool {
        self.register_shutdown_hook
    }

    /// Sets whether a shutdown hook is registered.
    pub fn set_register_shutdown_hook(&mut self, register_shutdown_hook: bool) {
        self.register_shutdown_hook = register_shutdown_hook;
    }

    /// Returns the sources to include in the ApplicationContext.
    pub fn get_sources(&self) -> &HashSet<String> {
        &self.sources
    }

    /// Sets the sources to include in the ApplicationContext.
    pub fn set_sources(&mut self, sources: HashSet<String>) {
        self.sources = sources;
    }

    /// Binds the `next.main.*` properties from the given environment onto this
    /// instance.
    ///
    /// Only properties that are actually present in the environment are
    /// applied; every other field keeps its current value.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment to resolve properties from.
    pub(crate) fn bind(&mut self, environment: &dyn ConfigurableEnvironment) {
        if let Some(value) = environment.get_property(ALLOW_OVERRIDE) {
            if let Some(allow_override) = parse_bool(&value) {
                self.allow_override = allow_override;
            }
        }

        if let Some(value) = environment.get_property(BANNER_MODE) {
            if let Ok(banner_mode) = value.parse::<BannerMode>() {
                self.banner_mode = banner_mode;
            }
        }

        if let Some(value) = environment.get_property(LOG_STARTUP_INFO) {
            if let Some(log_startup_info) = parse_bool(&value) {
                self.log_startup_info = log_startup_info;
            }
        }

        if let Some(value) = environment.get_property(REGISTER_SHUTDOWN_HOOK) {
            if let Some(register_shutdown_hook) = parse_bool(&value) {
                self.register_shutdown_hook = register_shutdown_hook;
            }
        }

        if let Some(value) = environment.get_property(SOURCES) {
            let sources = value
                .split(',')
                .map(str::trim)
                .filter(|source| !source.is_empty())
                .map(ToString::to_string)
                .collect::<HashSet<_>>();

            if !sources.is_empty() {
                self.sources = sources;
            }
        }
    }
}

/// Parses a boolean property value, accepting `true`/`false`
/// case-insensitively and ignoring surrounding whitespace.
fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

impl Default for ApplicationProperties {
    fn default() -> Self {
        Self {
            allow_override: false,
            banner_mode: BannerMode::default(),
            log_startup_info: true,
            register_shutdown_hook: true,
            sources: HashSet::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use next_web_core::env::{
        ConfigurableEnvironment, ConfigurablePropertyResolver, Environment, MutablePropertySources,
        PropertyResolver,
    };

    use super::*;

    #[derive(Default)]
    struct MockEnvironment {
        properties: HashMap<String, String>,
        property_sources: MutablePropertySources,
    }

    impl MockEnvironment {
        fn with_property(mut self, key: &str, value: &str) -> Self {
            self.properties.insert(key.to_string(), value.to_string());
            self
        }
    }

    impl PropertyResolver for MockEnvironment {
        fn contains_property(&self, key: &str) -> bool {
            self.properties.contains_key(key)
        }

        fn get_property(&self, key: &str) -> Option<String> {
            self.properties.get(key).cloned()
        }

        fn get_property_or_default(&self, _key: &str, _default_value: &str) -> String {
            todo!()
        }

        fn get_required_property(
            &self,
            _key: &str,
        ) -> Result<String, next_web_core::error::IllegalError> {
            todo!()
        }

        fn resolve_placeholders(&self, _text: &str) -> String {
            todo!()
        }

        fn resolve_required_placeholders(
            &self,
            _text: &str,
        ) -> Result<String, next_web_core::error::IllegalError> {
            todo!()
        }
    }

    impl ConfigurablePropertyResolver for MockEnvironment {}

    impl Environment for MockEnvironment {
        fn active_profiles(&self) -> &[String] {
            &[]
        }

        fn default_profiles(&self) -> &[String] {
            &[]
        }

        fn accepts_profiles(&self, _profiles: &dyn next_web_core::env::Profiles) -> bool {
            false
        }
    }

    impl ConfigurableEnvironment for MockEnvironment {
        fn set_active_profiles(&mut self, _profiles: &[&str]) {}

        fn add_active_profile(&mut self, _profile: &str) {}

        fn set_default_profiles(&mut self, _profiles: &[&str]) {}

        fn property_sources(&mut self) -> &mut MutablePropertySources {
            &mut self.property_sources
        }

        fn property_sources_ref(&self) -> &MutablePropertySources {
            &self.property_sources
        }

        fn system_properties(&self) -> HashMap<String, String> {
            HashMap::new()
        }

        fn system_environment(&self) -> HashMap<String, String> {
            HashMap::new()
        }

        fn merge(&mut self, parent: &dyn ConfigurableEnvironment) {}
    }

    #[test]
    fn bind_keeps_defaults_when_properties_absent() {
        let environment = MockEnvironment::default();
        let mut properties = ApplicationProperties::default();

        properties.bind(&environment);

        assert!(!properties.is_allow_override());
        assert_eq!(properties.get_banner_mode(), BannerMode::Console);
        assert!(properties.is_log_startup_info());
        assert!(properties.is_register_shutdown_hook());
        assert!(properties.get_sources().is_empty());
    }

    #[test]
    fn bind_applies_present_properties() {
        let environment = MockEnvironment::default()
            .with_property(ALLOW_OVERRIDE, "true")
            .with_property(BANNER_MODE, "LOG")
            .with_property(LOG_STARTUP_INFO, "false")
            .with_property(REGISTER_SHUTDOWN_HOOK, "false");

        let mut properties = ApplicationProperties::default();
        properties.bind(&environment);

        assert!(properties.is_allow_override());
        assert_eq!(properties.get_banner_mode(), BannerMode::Log);
        assert!(!properties.is_log_startup_info());
        assert!(!properties.is_register_shutdown_hook());
    }

    #[test]
    fn bind_parses_sources_list() {
        let environment =
            MockEnvironment::default().with_property(SOURCES, "first, second , ,third");

        let mut properties = ApplicationProperties::default();
        properties.bind(&environment);

        let expected: HashSet<String> = ["first", "second", "third"]
            .iter()
            .map(|source| source.to_string())
            .collect();
        assert_eq!(properties.get_sources(), &expected);
    }

    #[test]
    fn bind_ignores_invalid_values() {
        let environment = MockEnvironment::default()
            .with_property(ALLOW_OVERRIDE, "not-a-bool")
            .with_property(BANNER_MODE, "sideways");

        let mut properties = ApplicationProperties::default();
        properties.bind(&environment);

        assert!(!properties.is_allow_override());
        assert_eq!(properties.get_banner_mode(), BannerMode::Console);
    }
}
