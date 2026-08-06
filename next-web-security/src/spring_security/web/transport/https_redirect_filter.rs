use std::sync::Arc;

use next_web_core::{
    async_trait,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use crate::web::{
    port_mapper::PortMapper,
    util::matcher::{AnyRequestMatcher, RequestMatcher},
    DefaultRedirectStrategy, PortMapperImpl, RedirectStrategy,
};

/// Redirects HTTP requests to HTTPS, optionally using a `PortMapper`
/// to determine the correct HTTPS port.
#[derive(Clone)]
pub struct HttpsRedirectFilter {
    request_matcher: Arc<dyn RequestMatcher>,
    port_mapper: Arc<dyn PortMapper>,
    redirect_strategy: Arc<dyn RedirectStrategy>,
}

impl HttpsRedirectFilter {
    /// Use this `PortMapper` for mapping custom ports.
    ///
    /// # Arguments
    ///
    /// * `port_mapper` - The `PortMapper` to use.
    pub fn set_port_mapper(&mut self, port_mapper: Arc<dyn PortMapper>) {
        self.port_mapper = port_mapper;
    }

    /// Use this `RequestMatcher` to narrow which requests are redirected to HTTPS.
    ///
    /// The filter already first checks for HTTPS in the URI scheme, so it is not
    /// necessary to include that check in this matcher.
    ///
    /// # Arguments
    ///
    /// * `request_matcher` - The `RequestMatcher` to use.
    pub fn set_request_matcher(&mut self, request_matcher: Arc<dyn RequestMatcher>) {
        self.request_matcher = request_matcher;
    }

    /// Checks whether the request is insecure (i.e., not using HTTPS).
    ///
    /// # Arguments
    ///
    /// * `request` - The HTTP request to check.
    fn is_insecure(&self, request: &dyn HttpRequest) -> bool {
        request.scheme() != Some("https")
    }

    /// Creates the HTTPS redirect URI for the given request.
    ///
    /// Builds the full request URL, then replaces the scheme with "https" and updates
    /// the port using the configured `PortMapper` if a non-standard HTTP port is in use.
    ///
    /// # Arguments
    ///
    /// * `request` - The HTTP request to build the redirect URI from.
    ///
    /// # Panics
    ///
    /// Panics if the HTTP port does not have a corresponding HTTPS port mapping.
    fn create_redirect_uri(&self, request: &dyn HttpRequest) -> String {
        let host = request.uri().host().unwrap_or("localhost");

        // Determine the port to use in the redirect UR
        let https_port = request
            .uri()
            .port_u16()
            .filter(|&p| p > 0)
            .map(|http_port| {
                // Look up the corresponding HTTPS port
                self.port_mapper
                    .lookup_https_port(http_port)
                    .unwrap_or_else(|| {
                        panic!(
                            "HTTP Port '{}' does not have a corresponding HTTPS Port",
                            http_port
                        )
                    })
            })
            .unwrap_or(443);

        let path_and_query = request
            .uri()
            .path_and_query()
            .map(|paq| paq.as_str())
            .unwrap_or("/");
        // Omit port if it's the default HTTPS port (443)
        if https_port == 443 {
            format!("https://{}{}", host, path_and_query)
        } else {
            format!("https://{}:{}{}", host, https_port, path_and_query)
        }
    }
}

impl Default for HttpsRedirectFilter {
    fn default() -> Self {
        Self {
            request_matcher: AnyRequestMatcher::instance(),
            port_mapper: Arc::new(PortMapperImpl::default()),
            redirect_strategy: Arc::new(DefaultRedirectStrategy::default()),
        }
    }
}

#[async_trait]
impl HttpFilter for HttpsRedirectFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        // If the request is already secure, skip redirection.
        if !self.is_insecure(request) {
            return filter_chain.do_filter(request, response).await;
        }
        if !self.request_matcher.matches(request) {
            return filter_chain.do_filter(request, response).await;
        }
        let redirect_uri = self.create_redirect_uri(request);
        self.redirect_strategy
            .send_redirect(request, response, &redirect_uri)?;

        Ok(())
    }
}

impl Named for HttpsRedirectFilter {
    fn name(&self) -> &str {
        "HttpsRedirectFilter"
    }
}
