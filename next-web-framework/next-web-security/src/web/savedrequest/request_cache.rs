use axum::extract::Request;

use crate::web::savedrequest::SavedRequest;

pub trait RequestCache: Send + Sync
{
    fn save_request(&self, request: & Request);

    fn get_request(&self, request: & Request) -> Option<&dyn SavedRequest>;

    fn get_matching_request(&self, request: & Request) -> Option<& Request>;

    fn remove_request(&self, request: & Request);
}