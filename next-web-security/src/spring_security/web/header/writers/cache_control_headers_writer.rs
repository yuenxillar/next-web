use axum::http::StatusCode;

use crate::web::header::HeaderWriter;

#[derive(Default)]
pub struct CacheControlHeadersWriter;

const EXPIRES: &'static str = "Expires";
const PRAGMA: &'static str = "Pragma";
const CACHE_CONTROL: &'static str = "Cache-Control";

impl CacheControlHeadersWriter {
    pub fn has_header(
        &self,
        response: &mut dyn next_web_core::traits::http::http_response::HttpResponse,
    ) -> bool {
        response.contains_header(CACHE_CONTROL)
            || response.contains_header(EXPIRES)
            || response.contains_header(PRAGMA)
            || response.status_code() != StatusCode::NOT_MODIFIED
    }

    pub fn write_header(
        &self,
        response: &mut dyn next_web_core::traits::http::http_response::HttpResponse,
    ) {
        if response.contains_header(CACHE_CONTROL) {
            response.append_header(
                CACHE_CONTROL,
                "no-cache, no-store, max-age=0, must-revalidate",
            );
        }
        if response.contains_header(PRAGMA) {
            response.append_header(PRAGMA, "no-cache");
        }
        if response.contains_header(EXPIRES) {
            response.append_header(EXPIRES, "0");
        }
    }
}
impl HeaderWriter for CacheControlHeadersWriter {
    fn write_headers(
        &self,
        _request: &dyn next_web_core::traits::http::http_request::HttpRequest,
        response: &mut dyn next_web_core::traits::http::http_response::HttpResponse,
    ) {
        if self.has_header(response) {
            return;
        }

        self.write_header(response);
    }
}
