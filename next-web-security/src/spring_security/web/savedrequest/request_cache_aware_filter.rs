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

use crate::web::savedrequest::{HttpSessionRequestCache, RequestCache};

/// Responsible for reconstituting the saved request if one is cached and it matches the current request.
/// It will call get_matching_request on the configured RequestCache. If the method returns a value (a
/// wrapper of the saved request), it will pass this to the filter chain's doFilter method. If null is returned by
/// the cache, the original request is used and the filter has no effect.
#[derive(Clone)]
pub struct RequestCacheAwareFilter {
    request_cache: Arc<dyn RequestCache>,
}

impl RequestCacheAwareFilter {
    pub fn new(request_cache: Arc<dyn RequestCache>) -> Self {
        Self {
            request_cache: request_cache,
        }
    }
}

#[async_trait]
impl HttpFilter for RequestCacheAwareFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        let mut wrapped_saved_request = self.request_cache.get_matching_request(request, response);
        if let Some(wrapped_request) = wrapped_saved_request.as_mut() {
            filter_chain
                .do_filter(&mut **wrapped_request, response)
                .await
        } else {
            drop(wrapped_saved_request);

            filter_chain.do_filter(request, response).await
        }
    }
}

impl Named for RequestCacheAwareFilter {
    fn name(&self) -> &str {
        "RequestCacheAwareFilter"
    }
}

impl Default for RequestCacheAwareFilter {
    fn default() -> Self {
        Self {
            request_cache: Arc::new(HttpSessionRequestCache::default()),
        }
    }
}
