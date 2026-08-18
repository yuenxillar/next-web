use std::sync::Arc;

use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::web::savedrequest::{RequestCache, SavedRequest};

/// Null implementation of RequestCache. Typically used when creation of a session is not desired.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullRequestCache;

impl RequestCache for NullRequestCache {
    #[allow(unused_variables)]
    fn save_request(&self, request: &mut dyn HttpRequest, response: &mut dyn HttpResponse) {}

    #[allow(unused_variables)]
    fn get_request(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Option<Arc<dyn SavedRequest>> {
        None
    }

    #[allow(unused_variables)]
    fn remove_request(&self, request: &dyn HttpRequest, response: &mut dyn HttpResponse) {}

    #[allow(unused_variables)]
    fn get_matching_request<'a>(
        &self,
        request: &'a mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Option<Box<dyn HttpRequest + 'a>> {
        None
    }
}
