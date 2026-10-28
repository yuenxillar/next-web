//! The policy that allows the resources a web application serves.

use crate::io::{ResourceAccessPolicy, allows};

/// A policy that allows the resources a web application serves.
///
/// The policy covers the configuration data and the messages of an application,
/// the documents and the templates it renders, and the static assets it answers
/// with: stylesheets, scripts, images, fonts, and media. It is the policy the
/// shared resource loader uses, so an application can serve those files without
/// configuring anything.
///
/// A loader that must only read the configuration of an application uses
/// [`ConfigOnlyPolicy`](crate::io::ConfigOnlyPolicy) instead.
///
/// # Extending the policy
///
/// A location has to be allowed by every policy registered with a loader, so a
/// policy can only narrow what the loader reads; a loader with a resource of
/// its own uses
/// [`set_access_policies`](crate::io::DefaultResourceLoader::set_access_policies)
/// to register the policies it needs.
pub struct WebResourcePolicy {
    /// The extensions of the files the policy allows.
    allowed_extensions: Vec<&'static str>,
    /// The names of the files the policy allows whatever their extension.
    allowed_names: Vec<&'static str>,
}

impl WebResourcePolicy {
    /// Creates a policy with the resources a web application serves.
    pub fn new() -> Self {
        Self {
            allowed_extensions: vec![
                // The configuration data and the messages of an application.
                "yml",
                "yaml",
                "properties",
                "toml",
                "conf",
                "cfg",
                "ini",
                // The documents and the templates of an application.
                "html",
                "htm",
                "xml",
                "json",
                "txt",
                "md",
                "csv",
                // The stylesheets, the scripts, and their source maps.
                "css",
                "js",
                "mjs",
                "cjs",
                "map",
                // The images.
                "svg",
                "png",
                "jpg",
                "jpeg",
                "gif",
                "webp",
                "avif",
                "bmp",
                "ico",
                // The fonts.
                "woff",
                "woff2",
                "ttf",
                "otf",
                "eot",
                // The media.
                "mp3",
                "mp4",
                "webm",
                "wav",
                "ogg",
                // Whatever else a page links to.
                "pdf",
                "wasm",
            ],
            allowed_names: vec!["application", "config", "banner", ".env"],
        }
    }

    /// Adds an allowed extension to the policy.
    pub fn add_allowed_extension(&mut self, extension: &'static str) {
        self.allowed_extensions.push(extension);
    }

    /// Adds an allowed name to the policy.
    pub fn add_allowed_name(&mut self, name: &'static str) {
        self.allowed_names.push(name);
    }

    /// Returns the list of allowed extensions.
    pub fn allowed_extensions(&self) -> &[&'static str] {
        &self.allowed_extensions
    }

    /// Returns the list of allowed names.
    pub fn allowed_names(&self) -> &[&'static str] {
        &self.allowed_names
    }
}

impl Default for WebResourcePolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceAccessPolicy for WebResourcePolicy {
    /// Returns `true` if the given location names a resource of a web
    /// application.
    fn is_allowed(&self, location: &str) -> bool {
        allows(location, &self.allowed_extensions, &self.allowed_names)
    }
}

#[cfg(test)]
mod tests {
    use super::WebResourcePolicy;
    use crate::io::{ConfigOnlyPolicy, ResourceAccessPolicy};

    #[test]
    fn the_static_assets_of_a_page_are_allowed() {
        let policy = WebResourcePolicy::new();

        for location in [
            "static/app.css",
            "static/app.min.js",
            "static/logo.png",
            "static/icon.svg",
            "static/font.woff2",
            "static/clip.mp4",
        ] {
            assert!(policy.is_allowed(location), "{location} is not allowed");
        }
    }

    #[test]
    fn the_configuration_and_the_templates_are_allowed() {
        let policy = WebResourcePolicy::new();

        for location in [
            "application.yaml",
            "messages/messages_zh_CN.properties",
            "views/login.html",
            "hello.json",
        ] {
            assert!(policy.is_allowed(location), "{location} is not allowed");
        }
    }

    #[test]
    fn a_file_that_is_not_a_resource_of_a_page_is_refused() {
        let policy = WebResourcePolicy::new();

        assert!(!policy.is_allowed("tool.exe"));
        assert!(!policy.is_allowed("script.sh"));
        assert!(!policy.is_allowed("../application.yaml"));
    }

    #[test]
    fn an_extension_can_be_added_to_the_policy() {
        let mut policy = WebResourcePolicy::new();
        assert!(!policy.is_allowed("data.json5"));

        policy.add_allowed_extension("json5");

        assert!(policy.is_allowed("data.json5"));
    }

    /// The policy of a loader that only reads configuration is still narrower,
    /// which is what keeps it available for an application that wants it.
    #[test]
    fn the_configuration_only_policy_does_not_allow_the_static_assets() {
        let policy = ConfigOnlyPolicy::new();

        assert!(policy.is_allowed("application.yaml"));
        assert!(!policy.is_allowed("static/app.css"));
        assert!(!policy.is_allowed("static/logo.png"));
    }
}
