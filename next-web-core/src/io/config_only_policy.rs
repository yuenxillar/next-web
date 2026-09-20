use std::path::Path;

use crate::io::ResourceAccessPolicy;

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
    /// The check is performed against the path portion of the location,
    /// ignoring any known protocol prefix such as `classpath:` or `file:`.
    /// A location is allowed when either:
    ///
    /// 1. its file stem matches one of the allowed names (for example,
    ///    `application` for `application.yml`, or `.env` for `.env.local`), or
    /// 2. its file extension matches one of the allowed extensions,
    ///    compared case-insensitively.
    ///
    /// Any other location, including one with no recognizable file name or
    /// extension, is rejected.
    fn is_allowed(&self, location: &str) -> bool {
        // Normalize separators and reject any path that attempts to escape
        // its base directory via `..`.
        let normalized = location.replace('\\', "/");
        if normalized.split('/').any(|segment| segment == "..") {
            return false;
        }

        let path = Path::new(&normalized);

        // 1. Check whether the file stem matches one of the allowed names.
        //    This covers configuration files that either carry no extension
        //    at all (`config`) or are known by their base name regardless of
        //    extension (`application.yml`).
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            if self
                .allowed_names
                .iter()
                .any(|name| stem.eq_ignore_ascii_case(name))
            {
                return true;
            }
        }

        // 2. Check whether the file extension matches one of the allowed
        //    extensions, compared case-insensitively.
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            return self
                .allowed_extensions
                .iter()
                .any(|allowed| ext.eq_ignore_ascii_case(allowed));
        }

        false
    }
}
