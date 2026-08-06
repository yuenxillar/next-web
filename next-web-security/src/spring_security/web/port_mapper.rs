use next_web_core::traits::http::http_request::HttpRequest;

/// Maps HTTP ports to HTTPS ports and vice versa for redirects.
/// Used by `HttpsRedirectFilter` to determine the correct port when redirecting.
pub trait PortMapper
where
    Self: Send + Sync,
{
    /// Look up the HTTPS port for a given HTTP port.
    fn lookup_https_port(&self, https_port: u16) -> Option<u16>;

    /// Look up the HTTP port for a given HTTPS port.
    fn lookup_http_port(&self, http_port: u16) -> Option<u16>;

    /// Get server port from request and automatically apply the configured mapping.
    fn get_server_port(&self, request: &dyn HttpRequest) -> u16 {
        let server_port = request.server_port().unwrap_or(80);
        let scheme = request.scheme().unwrap_or_default();

        let mut mapped_port = None;
        if scheme == "http" {
            mapped_port = self.lookup_http_port(server_port);
        } else if scheme == "https" {
            mapped_port = self.lookup_https_port(server_port);
        }

        mapped_port.unwrap_or(server_port)
    }
}
