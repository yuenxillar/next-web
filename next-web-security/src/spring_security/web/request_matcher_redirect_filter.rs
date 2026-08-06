use std::sync::Arc;

use next_web_core::async_trait;
use next_web_core::filter::FilterError;
use next_web_core::traits::filter::{HttpFilter, HttpFilterChain};
use next_web_core::traits::http::http_request::HttpRequest;
use next_web_core::traits::http::http_response::HttpResponse;
use next_web_core::traits::named::Named;

use crate::web::util::matcher::RequestMatcher;
use crate::web::{DefaultRedirectStrategy, RedirectStrategy};

/// Filter that redirects requests that match a `RequestMatcher` to the specified URL.
#[derive(Clone)]
pub struct RequestMatcherRedirectFilter {
    redirect_strategy: Arc<dyn RedirectStrategy>,
    request_matcher: Arc<dyn RequestMatcher>,
    redirect_url: String,
}

impl RequestMatcherRedirectFilter {
    /// Create and initialize an instance of the filter.
    ///
    /// # Arguments
    ///
    /// * `request_matcher` - The request matcher used to determine which requests
    ///   should be redirected. Cannot be null.
    /// * `redirect_url` - The URL to redirect matching requests to. Cannot be empty.
    pub fn new(request_matcher: Arc<dyn RequestMatcher>, redirect_url: &str) -> Self {
        assert!(!redirect_url.is_empty(), "redirect_url cannot be empty");
        Self {
            redirect_strategy: Arc::new(DefaultRedirectStrategy::default()),
            request_matcher,
            redirect_url: redirect_url.to_string(),
        }
    }
}

#[async_trait]
impl HttpFilter for RequestMatcherRedirectFilter {
    /// Processes the filter logic: if the request matches the configured
    /// `RequestMatcher`, it redirects to the configured URL; otherwise, the request
    /// continues down the filter chain.
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if self.request_matcher.matches(request) {
            self.redirect_strategy
                .send_redirect(request, response, &self.redirect_url)?;
        } else {
            filter_chain.do_filter(request, response).await?;
        }
        Ok(())
    }
}

impl Named for RequestMatcherRedirectFilter {
    fn name(&self) -> &str {
        "RequestMatcherRedirectFilter"
    }
}
