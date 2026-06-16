use crate::web::header::{Header, HeaderWriter};

#[derive(Clone)]
pub struct StaticHeadersWriter {
    headers: Vec<Header>,
}

impl StaticHeadersWriter {
    pub fn new(headers: Vec<Header>) -> Self {
        Self { headers }
    }

    pub fn with(name: impl Into<String>, values: Vec<String>) -> Self {
        Self::new(vec![Header::new(name.into(), values)])
    }
}

impl HeaderWriter for StaticHeadersWriter {
    fn write_headers(
        &self,
        _request: &dyn next_web_core::traits::http::http_request::HttpRequest,
        response: &mut dyn next_web_core::traits::http::http_response::HttpResponse,
    ) {
        self.headers.iter().for_each(|header| {
            let header_name = header.header_name();
            if !response.contains_header(header_name) {
                header.header_value().iter().for_each(|value| {
                    response.append_header(header_name, value);
                });
            }
        });
    }
}
