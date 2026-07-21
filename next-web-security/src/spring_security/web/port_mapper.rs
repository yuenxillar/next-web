/// Maps HTTP ports to HTTPS ports and vice versa for redirects.
/// Used by `HttpsRedirectFilter` to determine the correct port when redirecting.
pub trait PortMapper
where
    Self: Send + Sync,
{
    /// Look up the HTTPS port for a given HTTP port.
    fn lookup_https_port(&self, http_port: u16) -> Option<u16>;

    /// Look up the HTTP port for a given HTTPS port.
    fn lookup_http_port(&self, https_port: u16) -> Option<u16>;
}
