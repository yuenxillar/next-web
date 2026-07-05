use std::fmt;
use std::sync::Arc;

use next_web_core::traits::http::http_request::HttpRequest;

use crate::authorization::SingleResultAuthorizationManager;
use crate::config::web::configurers::AuthorizeHttpRequestsConfigurer;
use crate::config::web::http_security_builder::HttpSecurityBuilder;
use crate::web::util::matcher::RequestMatcher;

/// Configures non-null URL's to grant access to every URL
///
/// This is a utility struct that provides static methods to configure
/// permit-all access for specific URLs in the security configuration.
pub struct PermitAllSupport;

impl PermitAllSupport {
    /// Configures permit-all access for the given URL strings.
    ///
    /// # Arguments
    ///
    /// * `http` - The HTTP security builder to configure
    /// * `urls` - The URL strings to permit all access
    pub fn permit_all<H>(http: &mut H, urls: &[&str])
    where
        H: HttpSecurityBuilder<H>,
        H: 'static,
    {
        for url in urls {
            if !url.is_empty() {
                Self::permit_all_with_matcher(
                    http,
                    &[Arc::new(ExactUrlRequestMatcher::new(url.to_string()))],
                );
            }
        }
    }

    /// Configures permit-all access for the given request matchers.
    ///
    /// # Arguments
    ///
    /// * `http` - The HTTP security builder to configure
    /// * `request_matchers` - The request matchers to permit all access
    ///
    /// # Panics
    ///
    /// Panics if the HTTP security builder does not have an `AuthorizeHttpRequestsConfigurer` configured.
    pub fn permit_all_with_matcher<H>(http: &mut H, request_matchers: &[Arc<dyn RequestMatcher>])
    where
        H: HttpSecurityBuilder<H>,
        H: 'static,
    {
        let http_configurer = http
            .configurer_mut::<AuthorizeHttpRequestsConfigurer<H>>()
            .expect(
            "permitAll only works with HttpSecurity.authorizeHttpRequests(). Please define one.",
        );

        for matcher in request_matchers {
            http_configurer.add_first(
                matcher.clone(),
                Arc::new(SingleResultAuthorizationManager::permit_all()),
            );
        }
    }
}

/// A request matcher that performs exact URL matching.
///
/// This matcher compares the full request URI (including query string)
/// against a configured process URL, taking into account the context path.
#[derive(Debug)]
struct ExactUrlRequestMatcher {
    process_url: String,
}

impl ExactUrlRequestMatcher {
    /// Creates a new `ExactUrlRequestMatcher` with the given process URL.
    ///
    /// # Arguments
    ///
    /// * `process_url` - The URL pattern to match against
    fn new(process_url: String) -> Self {
        Self { process_url }
    }
}

impl RequestMatcher for ExactUrlRequestMatcher {
    /// Determines if the given request matches the configured process URL.
    ///
    /// This method compares the request URI (including query string if present)
    /// against the configured process URL. If the context path is empty, it
    /// compares directly; otherwise, it prepends the context path.
    ///
    /// # Arguments
    ///
    /// * `request` - The HTTP request to match against
    ///
    /// # Returns
    ///
    /// `true` if the request matches the exact URL, `false` otherwise
    fn matches(&self, request: &dyn HttpRequest) -> bool {
        let mut uri = request.path().to_string();
        let query = request.query();

        // Append query string if present
        if let Some(query) = query {
            if !query.is_empty() {
                uri.push('?');
                uri.push_str(query);
            }
        }

        let context_path = request.context_path().unwrap_or_default();

        // If context path is empty, compare URI directly
        if context_path.is_empty() {
            return uri == self.process_url;
        }

        // Otherwise, prepend context path for comparison
        uri == format!("{}{}", context_path, self.process_url)
    }
}

impl fmt::Display for ExactUrlRequestMatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ExactUrl [processUrl='{}']", self.process_url)
    }
}
