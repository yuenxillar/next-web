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
        request: &dyn next_web_core::http::HttpRequest,
        response: &mut dyn next_web_core::http::HttpResponse,
    ) {
        self.0.write_headers(request, response);
    }
}
