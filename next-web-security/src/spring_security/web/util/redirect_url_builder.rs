/// Internal class for building redirect URLs.
///
/// Could probably make more use of the classes in `std::net` for this.
pub struct RedirectUrlBuilder {
    scheme: Option<String>,
    server_name: Option<String>,
    port: u16,
    context_path: Option<String>,
    path: Option<String>,
    query: Option<String>,
}

impl RedirectUrlBuilder {
    /// Creates a new instance with all fields unset.
    pub fn new() -> Self {
        Self {
            scheme: None,
            server_name: None,
            port: 0,
            context_path: None,
            path: None,
            query: None,
        }
    }

    /// Sets the URL scheme (protocol).
    ///
    /// # Arguments
    /// * `scheme` - the scheme, must be either "http" or "https"
    ///
    /// # Panics
    /// Panics if the scheme is not "http" or "https".
    pub fn set_scheme(&mut self, scheme: impl Into<String>) {
        let scheme = scheme.into();
        assert!(
            scheme == "http" || scheme == "https",
            "Unsupported scheme '{}'",
            scheme
        );

        self.scheme = Some(scheme);
    }

    /// Sets the server name for the URL.
    ///
    /// # Arguments
    /// * `server_name` - the server name (e.g., "example.com")
    pub fn set_server_name(&mut self, server_name: String) {
        self.server_name = Some(server_name);
    }

    /// Sets the port number for the URL.
    ///
    /// # Arguments
    /// * `port` - the port number
    pub fn set_port(&mut self, port: u16) {
        self.port = port;
    }

    /// Sets the context path for the URL.
    ///
    /// # Arguments
    /// * `context_path` - the context path (e.g., "/myapp")
    pub fn set_context_path(&mut self, context_path: impl Into<String>) {
        self.context_path = Some(context_path.into());
    }

    /// Sets the servlet path for the URL.
    ///
    /// # Arguments
    /// * `path` - the path (e.g., "/login")
    pub fn set_path(&mut self, path: impl Into<String>) {
        self.path = Some(path.into());
    }

    /// Sets the query string for the URL.
    ///
    /// # Arguments
    /// * `query` - the query string (without the leading '?')
    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = Some(query.into());
    }

    /// Builds and returns the complete redirect URL from the configured components.
    ///
    /// # Returns
    /// The constructed URL string.
    ///
    /// # Panics
    /// Panics if `scheme` or `server_name` have not been set.
    pub fn get_url(&self) -> String {
        let scheme = self.scheme.as_ref().expect("scheme cannot be null");
        let server_name = self
            .server_name
            .as_ref()
            .expect("serverName cannot be null");

        let mut url = format!("{}://{}", scheme, server_name);

        // Append the port number if it's not standard for the scheme
        let standard_port = if scheme == "http" { 80 } else { 443 };
        if self.port != 0 && self.port != standard_port {
            url.push_str(&format!(":{}", self.port));
        }

        if let Some(ref context_path) = self.context_path {
            url.push_str(context_path);
        }

        if let Some(ref path) = self.path {
            url.push_str(path);
        }

        if let Some(ref query) = self.query {
            url.push('?');
            url.push_str(query);
        }

        url
    }

    /// Returns the configured scheme, if set.
    ///
    /// # Returns
    /// The scheme or `None` if not set.
    pub fn get_scheme(&self) -> Option<&str> {
        self.scheme.as_deref()
    }

    /// Returns the configured server name, if set.
    ///
    /// # Returns
    /// The server name or `None` if not set.
    pub fn get_server_name(&self) -> Option<&str> {
        self.server_name.as_deref()
    }

    /// Returns the configured port.
    ///
    /// # Returns
    /// The port number (0 means not explicitly set).
    pub fn get_port(&self) -> u16 {
        self.port
    }

    /// Returns the configured context path, if set.
    ///
    /// # Returns
    /// The context path or `None` if not set.
    pub fn get_context_path(&self) -> Option<&str> {
        self.context_path.as_deref()
    }

    /// Returns the configured servlet path, if set.
    ///
    /// # Returns
    /// The servlet path or `None` if not set.
    pub fn get_path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Returns the configured query string, if set.
    ///
    /// # Returns
    /// The query string (without leading '?') or `None` if not set.
    pub fn get_query(&self) -> Option<&str> {
        self.query.as_deref()
    }
}

impl Default for RedirectUrlBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url_with_standard_http_port() {
        let mut builder = RedirectUrlBuilder::new();
        builder.set_scheme("http".to_string());
        builder.set_server_name("example.com".to_string());
        builder.set_port(80);
        builder.set_context_path("/myapp".to_string());
        builder.set_path("/login".to_string());

        let url = builder.get_url();
        assert_eq!(url, "http://example.com/myapp/login");
    }

    #[test]
    fn test_build_url_with_standard_https_port() {
        let mut builder = RedirectUrlBuilder::new();
        builder.set_scheme("https".to_string());
        builder.set_server_name("example.com".to_string());
        builder.set_port(443);

        let url = builder.get_url();
        assert_eq!(url, "https://example.com");
    }

    #[test]
    fn test_build_url_with_non_standard_port() {
        let mut builder = RedirectUrlBuilder::new();
        builder.set_scheme("http".to_string());
        builder.set_server_name("example.com".to_string());
        builder.set_port(8080);

        let url = builder.get_url();
        assert_eq!(url, "http://example.com:8080");
    }

    #[test]
    fn test_build_url_with_query_string() {
        let mut builder = RedirectUrlBuilder::new();
        builder.set_scheme("https".to_string());
        builder.set_server_name("example.com".to_string());
        builder.set_port(443);
        builder.set_context_path("/app".to_string());
        builder.set_path("/search".to_string());
        builder.set_query("q=rust&page=1".to_string());

        let url = builder.get_url();
        assert_eq!(url, "https://example.com/app/search?q=rust&page=1");
    }

    #[test]
    fn test_build_url_with_path_info() {
        let mut builder = RedirectUrlBuilder::new();
        builder.set_scheme("http".to_string());
        builder.set_server_name("localhost".to_string());
        builder.set_port(8080);
        builder.set_context_path("/api".to_string());
        builder.set_path("/users/123".to_string());

        let url = builder.get_url();
        assert_eq!(url, "http://localhost:8080/api/users/123");
    }

    #[test]
    #[should_panic(expected = "Unsupported scheme")]
    fn test_invalid_scheme() {
        let mut builder = RedirectUrlBuilder::new();
        builder.set_scheme("ftp".to_string());
    }

    #[test]
    #[should_panic(expected = "scheme cannot be null")]
    fn test_missing_scheme() {
        let builder = RedirectUrlBuilder::new();
        builder.get_url();
    }

    #[test]
    #[should_panic(expected = "serverName cannot be null")]
    fn test_missing_server_name() {
        let mut builder = RedirectUrlBuilder::new();
        builder.set_scheme("http".to_string());
        builder.get_url();
    }
}
