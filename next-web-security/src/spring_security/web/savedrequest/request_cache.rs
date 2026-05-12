use std::sync::Arc;

use axum::extract::Request;

use crate::web::savedrequest::SavedRequest;

pub trait RequestCache: Send + Sync {
    fn save_request(&self, request: &Request);

    fn get_request(&self, request: &Request) -> Option<Arc<dyn SavedRequest>>;

    fn get_matching_request(&self, request: &Request) -> Option<Arc<dyn SavedRequest>>;

    fn remove_request(&self, request: &Request);
}
