use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

pub trait HeaderWriter: Send + Sync {
    fn write_headers(&self, request: &dyn HttpRequest, response: &mut dyn HttpResponse);
}
