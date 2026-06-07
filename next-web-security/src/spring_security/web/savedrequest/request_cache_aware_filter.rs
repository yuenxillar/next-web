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

use crate::web::savedrequest::RequestCache;

#[derive(Clone)]
pub struct RequestCacheAwareFilter {
    request_cache: Option<Arc<dyn RequestCache>>,
}

impl RequestCacheAwareFilter {
    pub fn new(request_cache: Arc<dyn RequestCache>) -> Self {
        Self {
            request_cache: Some(request_cache),
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
    ) -> Result<(), BoxError> {
        match self.request_cache.as_ref() {
            Some(cache) => {
                let mut wrapped_saved_request = cache.get_matching_request(request, response);
                filter_chain
                    .do_filter(
                        wrapped_saved_request.as_deref_mut().unwrap_or(request),
                        response,
                    )
                    .await
            }
            None => filter_chain.do_filter(request, response).await,
        }
    }
}

impl Named for RequestCacheAwareFilter {
    fn name(&self) -> &str {
        "RequestCacheAwareFilter"
    }
}
