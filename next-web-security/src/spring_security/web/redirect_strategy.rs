use next_web_core::http::StatusCode;
use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use tracing::{debug, enabled, Level};

use crate::web::util::UrlUtils;

pub trait RedirectStrategy: Send + Sync {
    fn send_redirect(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        url: &str,
    ) -> Result<(), BoxError>;
}

/// Simple implementation of RedirectStrategy which is the default used throughout the framework.
#[derive(Clone)]
pub struct DefaultRedirectStrategy {
    status_code: StatusCode,
    context_relative: bool,
}

impl Default for DefaultRedirectStrategy {
    fn default() -> Self {
        Self {
            status_code: StatusCode::FOUND,
            context_relative: false,
        }
    }
}

impl RedirectStrategy for DefaultRedirectStrategy {
    /// Redirects the response to the supplied URL.
    ///
    /// If `context_relative` is set, the redirect value will be the value after the
    /// request context path. Note that this will result in the loss of protocol
    /// information (HTTP or HTTPS), so will cause problems if a redirect is being
    /// performed to change to HTTPS, for example.
    fn send_redirect(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        url: &str,
    ) -> Result<(), BoxError> {
        let redirect_url =
            self.calculate_redirect_url(request.context_path().unwrap_or_default(), url);

        if enabled!(Level::DEBUG) {
            debug!("Redirecting to {}", redirect_url.as_str());
        }

        if self.status_code == StatusCode::FOUND {
            response.set_redirect(&redirect_url);
        } else {
            response.insert_header("location", redirect_url.as_str());
            response.set_status_code(self.status_code);
            response.finish();
        }

        Ok(())
    }
}

impl DefaultRedirectStrategy {
    /// Calculates the redirect URL based on the context path and whether
    /// context-relative mode is enabled.
    ///
    /// # Arguments
    /// * `context_path` - the request context path
    /// * `url` - the target URL
    ///
    /// # Returns
    /// The calculated redirect URL.
    ///
    /// # Panics
    /// Panics if the URL is fully qualified and context-relative mode is enabled,
    /// but the URL does not contain the context path.
    pub fn calculate_redirect_url(&self, context_path: &str, url: &str) -> String {
        if !UrlUtils::is_absolute_url(url) {
            if self.is_context_relative() {
                return url.to_string();
            }
            return format!("{}{}", context_path, url);
        }

        // Full URL, including http(s)://
        if !self.is_context_relative() {
            return url.to_string();
        }

        assert!(
            url.contains(context_path),
            "The fully qualified URL does not include context path."
        );

        // Calculate the relative URL from the fully qualified URL, minus the last
        // occurrence of the scheme and base context.
        let scheme_end = url.rfind("://").expect("Absolute URL must contain '://'");
        let mut relative = &url[scheme_end + 3..];

        let context_start = relative
            .find(context_path)
            .expect("Context path must be present in the URL");
        relative = &relative[context_start + context_path.len()..];

        if relative.len() > 1 && relative.starts_with('/') {
            relative = &relative[1..];
        }

        relative.to_string()
    }

    /// If `true`, causes any redirection URLs to be calculated minus the protocol
    /// and context path (defaults to `false`).
    ///
    /// # Arguments
    /// * `use_relative_context` - whether to use context-relative URLs
    pub fn set_context_relative(&mut self, use_relative_context: bool) {
        self.context_relative = use_relative_context;
    }

    /// Returns `true`, if the redirection URL should be calculated minus the
    /// protocol and context path (defaults to `false`).
    ///
    /// # Returns
    /// `true` if context-relative mode is enabled, `false` otherwise
    pub fn is_context_relative(&self) -> bool {
        self.context_relative
    }

    /// Sets the HTTP status code to use. The default is `HttpStatus::FOUND` (302).
    ///
    /// Note that according to RFC 7231, with `HttpStatus::FOUND`, a user agent MAY
    /// change the request method from POST to GET for the subsequent request. If this
    /// behavior is undesired, `HttpStatus::TEMPORARY_REDIRECT` can be used instead.
    ///
    /// # Arguments
    /// * `status_code` - the HTTP status code to use
    pub fn set_status_code(&mut self, status_code: StatusCode) {
        self.status_code = status_code;
    }

    /// Returns the currently configured HTTP status code.
    ///
    /// # Returns
    /// The HTTP status code used for redirects
    pub fn get_status_code(&self) -> StatusCode {
        self.status_code
    }
}
