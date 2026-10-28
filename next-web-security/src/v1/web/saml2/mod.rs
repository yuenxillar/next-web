//! The SAML 2.0 endpoints of the web layer.

use next_web_core::http::HttpRequest;

pub mod login;
pub mod logout;
pub mod metadata;

/// Returns the base URL of a request, without the path it was made on.
///
/// The value replaces the `{baseUrl}` placeholder of a registration field, so a
/// registration publishes the host, the port and the context path it is served
/// on.
pub(crate) fn request_base_url(request: &dyn HttpRequest) -> String {
    let scheme = request.scheme().unwrap_or("http");
    let host = request.host().unwrap_or_default();
    let port = request.server_port().unwrap_or_default();
    let default_port = matches!((scheme, port), ("http", 80) | ("https", 443));

    let mut base_url = if port == 0 || default_port {
        format!("{scheme}://{host}")
    } else {
        format!("{scheme}://{host}:{port}")
    };

    if let Some(context_path) = request.context_path() {
        base_url.push_str(context_path);
    }

    base_url
}
