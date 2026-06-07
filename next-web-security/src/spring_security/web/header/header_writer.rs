use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

pub trait HeaderWriter {
    fn write_headers(&self, request: &mut dyn HttpRequest, response: &mut dyn HttpResponse);
}
