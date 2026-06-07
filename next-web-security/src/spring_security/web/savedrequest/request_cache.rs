use std::sync::Arc;

use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::web::savedrequest::SavedRequest;

pub trait RequestCache
where
    Self: Send + Sync,
{
    fn save_request(&self, request: &dyn HttpRequest, response: &mut dyn HttpResponse);

    fn get_request(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Option<Arc<dyn SavedRequest>>;

    fn get_matching_request(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Option<Box<dyn HttpRequest>>;

    fn remove_request(&self, request: &dyn HttpRequest, response: &mut dyn HttpResponse);
}
