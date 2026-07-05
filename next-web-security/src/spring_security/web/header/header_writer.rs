use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

/// Contract for writing headers to a HttpResponse
pub trait HeaderWriter: Send + Sync {
    /// Writes the headers to the response.
    fn write_headers(&self, request: &dyn HttpRequest, response: &mut dyn HttpResponse);
}
