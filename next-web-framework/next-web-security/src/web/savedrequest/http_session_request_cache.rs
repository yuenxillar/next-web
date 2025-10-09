use crate::web::savedrequest::request_cache::RequestCache;

pub struct HttpSessionRequestCache {}

impl HttpSessionRequestCache {
    pub fn new() -> Self {
        Self {}
    }
}

impl RequestCache for HttpSessionRequestCache {
    fn save_request(&self, request: &axum::extract::Request) -> Result<String, String> {
        todo!()
    }

    fn get_request(&self, request: &axum::extract::Request) -> Option<String> {
        todo!()
    }

    fn get_matching_request(&self, request: &axum::extract::Request) {
        todo!()
    }

    fn remove_request(&self, request: &axum::extract::Request) {
        todo!()
    }
}
