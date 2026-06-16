use axum::http::StatusCode;
use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::web::header::{writers::StaticHeadersWriter, Header, HeaderWriter};

#[derive(Clone)]
pub struct CacheControlHeadersWriter {
    delegate: StaticHeadersWriter,
}

const EXPIRES: &'static str = "Expires";
const PRAGMA: &'static str = "Pragma";
const CACHE_CONTROL: &'static str = "Cache-Control";

impl CacheControlHeadersWriter {
    fn has_header(&self, response: &mut dyn HttpResponse, header_name: &str) -> bool {
        response.contains_header(header_name)
    }

    fn create_headers() -> [Header; 3] {
        [
            Header::new(
                CACHE_CONTROL.into(),
                vec!["no-cache, no-store, max-age=0, must-revalidate".into()],
            ),
            Header::new(PRAGMA.into(), vec!["no-cache".into()]),
            Header::new(EXPIRES.into(), vec!["0".into()]),
        ]
    }
}

impl HeaderWriter for CacheControlHeadersWriter {
    fn write_headers(&self, request: &dyn HttpRequest, response: &mut dyn HttpResponse) {
        if self.has_header(response, CACHE_CONTROL)
            || self.has_header(response, EXPIRES)
            || self.has_header(response, PRAGMA)
            || response.status_code() == StatusCode::NOT_MODIFIED
        {
            return;
        }

        self.delegate.write_headers(request, response);
    }
}

impl Default for CacheControlHeadersWriter {
    fn default() -> Self {
        Self {
            delegate: StaticHeadersWriter::new(Self::create_headers().to_vec()),
        }
    }
}
