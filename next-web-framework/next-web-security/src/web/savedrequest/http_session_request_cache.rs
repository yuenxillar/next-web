use axum::extract::Request;

use crate::web::savedrequest::request_cache::RequestCache;

use super::SavedRequest;

pub struct HttpSessionRequestCache {}

impl HttpSessionRequestCache {
    pub fn new() -> Self {
        Self {}
    }
}

impl RequestCache for HttpSessionRequestCache {
    fn save_request(&self, request: &axum::extract::Request) {
        todo!()
    }

    fn get_request(&self, request: &axum::extract::Request) -> Option<&dyn SavedRequest> {
        todo!()
    }

    fn get_matching_request(&self, request: &axum::extract::Request) ->Option<& Request> {
        todo!()
    }

    fn remove_request(&self, request: &axum::extract::Request) {
        todo!()
    }
}
