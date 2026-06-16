use crate::web::header::{writers::StaticHeadersWriter, HeaderWriter};

#[derive(Clone)]
pub struct XContentTypeOptionsHeaderWriter(StaticHeadersWriter);

impl Default for XContentTypeOptionsHeaderWriter {
    fn default() -> Self {
        Self(StaticHeadersWriter::with(
            "X-Content-Type-Options",
            vec!["nosniff".into()],
        ))
    }
}

impl HeaderWriter for XContentTypeOptionsHeaderWriter {
    fn write_headers(
        &self,
        _request: &dyn next_web_core::traits::http::http_request::HttpRequest,
        response: &mut dyn next_web_core::traits::http::http_response::HttpResponse,
    ) {
        let header_name = "X-Content-Type-Options";
        let header_value = "nosniff";

        if !response.contains_header(header_name) {
            response.append_header(header_name, header_value);
        }
    }
}
