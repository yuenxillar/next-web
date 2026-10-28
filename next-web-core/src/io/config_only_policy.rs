use crate::io::{ResourceAccessPolicy, allows};

/// A policy that only allows configuration files to be loaded.
pub struct ConfigOnlyPolicy {
    allowed_extensions: Vec<&'static str>,
    allowed_names: Vec<&'static str>,
}

impl ConfigOnlyPolicy {
    /// Create a new `ConfigOnlyPolicy` with the default set of allowed
    /// configuration file extensions and names.
    ///
    /// The allowed extensions cover the common configuration formats
    /// (`yml`, `yaml`, `properties`, `toml`, `json`, `conf`, `cfg`, `ini`),
    /// while the allowed names cover configuration files that typically
    /// carry no extension (`application`, `bootstrap`, `config`, `.env`).
    pub fn new() -> Self {
        Self {
            allowed_extensions: vec![
                "yml",
                "yaml",
                "properties",
                "xml",
                "toml",
                "json",
                "conf",
                "cfg",
                "ini",
                "html",
                "txt",
            ],
            // Some configuration files have no extension and are identified
            // solely by their well-known name.
            allowed_names: vec!["application", "config", ".env"],
        }
    }

    /// Add an allowed extension to the policy.
    pub fn add_allowed_extension(&mut self, ext: &'static str) {
        self.allowed_extensions.push(ext);
    }

    /// Add an allowed name to the policy.
    pub fn add_allowed_name(&mut self, name: &'static str) {
        self.allowed_names.push(name);
    }

    /// Return the list of allowed extensions.
    pub fn allowed_extensions(&self) -> &[&'static str] {
        &self.allowed_extensions
    }

    /// Return the list of allowed names.
    pub fn allowed_names(&self) -> &[&'static str] {
        &self.allowed_names
    }
}

impl Default for ConfigOnlyPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceAccessPolicy for ConfigOnlyPolicy {
    /// Return `true` if the given location points to a configuration file
    /// permitted by this policy.
    ///
    /// A location is allowed when its file stem is one of the allowed names
    /// (for example, `application` for `application.yml`, or `.env` for
    /// `.env.local`), or when its extension is one of the allowed extensions,
    /// compared case-insensitively. Any other location, including one with no
    /// recognizable file name or extension, is rejected.
    ///
    /// Use [`WebResourcePolicy`](crate::io::WebResourcePolicy) for a loader that
    /// also has to answer with the static assets of an application.
    fn is_allowed(&self, location: &str) -> bool {
        allows(location, &self.allowed_extensions, &self.allowed_names)
    }
}
