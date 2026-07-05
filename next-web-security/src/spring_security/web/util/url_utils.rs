use next_web_core::{
    constants::application_constants::APPLICATION_DEFAULT_PORT,
    traits::http::http_request::HttpRequest,
};
use regex::Regex;
use std::sync::LazyLock;

/// Provides static methods for composing URLs.
///
/// Placed into a separate module for visibility, so that changes to URL formatting
/// conventions will affect all users.
pub struct UrlUtils;

/// Compiled regex pattern for matching absolute URLs.
/// Matches URLs starting with a scheme name followed by "://", as defined in RFC 1738.
static ABSOLUTE_URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\A[a-z0-9.+-]+://.*").expect("Failed to compile absolute URL regex")
});

impl UrlUtils {
    pub fn build_full_request_url(req: &dyn HttpRequest) -> String {
        Self::_build_full_request_url(
            req.scheme().unwrap_or("http"),
            req.server_name().as_deref(),
            req.server_port().unwrap_or(APPLICATION_DEFAULT_PORT),
            req.path(),
            req.query(),
        )
    }

    /// Builds the full request URL from the components of an HTTP request.
    ///
    /// Note that the server port will not be shown if it is the default server port for
    /// HTTP or HTTPS (80 and 443 respectively).
    ///
    /// # Arguments
    ///
    /// * `scheme` - The URL scheme (e.g., "http" or "https")
    /// * `server_name` - The server name or IP address
    /// * `server_port` - The server port number
    /// * `request_uri` - The request URI (encoded per RFC 3986)
    /// * `query_string` - Optional query string (may be None)
    ///
    /// # Returns
    ///
    /// The full URL string, suitable for redirects (not decoded).
    fn _build_full_request_url(
        scheme: &str,
        server_name: Option<&str>,
        server_port: u16,
        request_uri: &str,
        query_string: Option<&str>,
    ) -> String {
        let scheme = scheme.to_lowercase();
        let mut url = String::new();
        url.push_str(&scheme);
        url.push_str("://");
        server_name.map(|name| url.push_str(name));

        // Only add port if not default
        if scheme == "http" {
            if server_port != 80 {
                url.push(':');
                url.push_str(&server_port.to_string());
            }
        } else if scheme == "https" {
            if server_port != 443 {
                url.push(':');
                url.push_str(&server_port.to_string());
            }
        }

        // Use the requestURI as it is encoded (RFC 3986) and hence suitable for redirects.
        url.push_str(request_uri);

        if let Some(qs) = query_string {
            url.push('?');
            url.push_str(qs);
        }

        url
    }

    pub fn build_request_url(req: &dyn HttpRequest) -> String {
        Self::_build_request_url(
            req.uri().path_and_query().map(|s| s.as_str()),
            req.context_path(),
        )
    }

    /// Obtains the web application-specific fragment of the request URL.
    ///
    /// Under normal spec conditions:
    ///
    /// ```text
    /// requestURI = contextPath + servletPath + pathInfo
    /// ```
    ///
    /// But the requestURI is not decoded, whereas the servletPath and pathInfo are
    /// (SEC-1255). This method is typically used to return a URL for matching against
    /// secured paths, hence the decoded form is used in preference to the requestURI for
    /// building the returned value. But this method may also be called using dummy request
    /// objects which just have the requestURI and contextPath set, for example, so it will
    /// fall back to using those.
    fn _build_request_url(path_and_query: Option<&str>, context_path: Option<&str>) -> String {
        match (context_path, path_and_query) {
            // 有 context_path，且有完整路径
            (Some(ctx_path), Some(path_and_query)) => {
                // 正确截取：从 ctx_path.len() 开始
                // 注意：ctx_path 包含开头的 '/'
                if path_and_query.starts_with(ctx_path) {
                    path_and_query[ctx_path.len()..].to_string()
                } else {
                    // 降级：如果路径不匹配，返回完整路径
                    path_and_query.to_string()
                }
            }
            // 无 context_path，有完整路径 → 直接返回
            (None, Some(path_and_query)) => path_and_query.to_string(),
            // 无路径 → 返回空字符串
            _ => String::new(),
        }
    }

    /// Returns true if the supplied URL starts with a "/" or is absolute.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to validate
    ///
    /// # Returns
    ///
    /// `true` if the URL is valid for redirect purposes
    pub fn is_valid_redirect_url(url: &str) -> bool {
        url.starts_with('/') || UrlUtils::is_absolute_url(url)
    }

    /// Decides if a URL is absolute based on whether it contains a valid scheme name, as
    /// defined in RFC 1738.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to check
    ///
    /// # Returns
    ///
    /// `true` if the URL is absolute
    pub fn is_absolute_url(url: &str) -> bool {
        ABSOLUTE_URL.is_match(url)
    }
}
