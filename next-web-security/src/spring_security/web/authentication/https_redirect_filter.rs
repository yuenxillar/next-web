use std::sync::Arc;

use next_web_core::{
    async_trait,
    error::BoxError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use crate::web::{
    port_mapper::PortMapper,
    util::matcher::{AnyRequestMatcher, RequestMatcher},
};

/// Redirects HTTP requests to HTTPS, optionally using a `PortMapper`
/// to determine the correct HTTPS port.
#[derive(Clone)]
pub struct HttpsRedirectFilter {
    request_matcher: Arc<dyn RequestMatcher>,
    port_mapper: Option<Arc<dyn PortMapper>>,
}

impl HttpsRedirectFilter {
    pub fn new() -> Self {
        Self {
            request_matcher: Arc::new(AnyRequestMatcher),
            port_mapper: None,
        }
    }

    /// Restrict which requests are redirected (default: all requests).
    pub fn set_request_matcher(&mut self, matcher: Arc<dyn RequestMatcher>) {
        self.request_matcher = matcher;
    }

    /// Set the port mapper for HTTP→HTTPS port lookups.
    pub fn set_port_mapper(&mut self, mapper: Arc<dyn PortMapper>) {
        self.port_mapper = Some(mapper);
    }

    /// Build the HTTPS redirect URL from the request URI.
    fn build_redirect_url(&self, request: &dyn HttpRequest) -> String {
        let host = request
            .header("Host")
            .unwrap_or("localhost");
        let uri = request.uri();
        let https_port = self.https_port(host);
        format!("https://{}{}", https_port, uri)
    }

    fn https_port(&self, host: &str) -> String {
        let (hostname, http_port) = parse_host_port(host);
        if http_port == 443 {
            return hostname.to_string();
        }
        let https_port = self
            .port_mapper
            .as_ref()
            .and_then(|m| m.lookup_https_port(http_port))
            .unwrap_or(443);
        if https_port == 443 {
            hostname.to_string()
        } else {
            format!("{}:{}", hostname, https_port)
        }
    }
}

impl Default for HttpsRedirectFilter {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl HttpFilter for HttpsRedirectFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), BoxError> {
        if !request.is_secure() && self.request_matcher.matches(request) {
            let url = self.build_redirect_url(request);
            response.set_redirect(&url);
            return Ok(());
        }
        filter_chain.do_filter(request, response).await
    }
}

impl Named for HttpsRedirectFilter {
    fn name(&self) -> &str { "HttpsRedirectFilter" }
}

fn parse_host_port(host: &str) -> (&str, u16) {
    if let Some(idx) = host.rfind(':') {
        if let Ok(port) = host[idx + 1..].parse::<u16>() {
            return (&host[..idx], port);
        }
    }
    // Default: assume standard ports
    (host, 80)
}
