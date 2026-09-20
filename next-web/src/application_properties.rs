//! Application properties.

use std::collections::HashSet;

use crate::banner::BannerMode;

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
