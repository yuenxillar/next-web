use std::sync::Arc;

use next_web_core::{
    async_trait,
    error::BoxError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use crate::web::header::HeaderWriter;

/// A filter that writes security headers to the response.
/// By default, headers are written before the filter chain executes.
#[derive(Clone)]
pub struct HeaderWriterFilter {
    header_writers: Vec<Arc<dyn HeaderWriter>>,
}

impl HeaderWriterFilter {
    pub fn new(header_writers: Vec<Arc<dyn HeaderWriter>>) -> Self {
        Self { header_writers }
    }

    pub fn add_header_writer(&mut self, header_writer: Arc<dyn HeaderWriter>) {
        self.header_writers.push(header_writer);
    }

    pub fn clear_header_writers(&mut self) {
        self.header_writers.clear();
    }

    fn write_headers(&self, request: &dyn HttpRequest, response: &mut dyn HttpResponse) {
        for writer in &self.header_writers {
            writer.write_headers(request, response);
        }
    }
}

#[async_trait]
impl HttpFilter for HeaderWriterFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), BoxError> {
        // Write security headers before the filter chain executes
        self.write_headers(request as &dyn HttpRequest, response);
        filter_chain.do_filter(request, response).await
    }
}

impl Named for HeaderWriterFilter {
    fn name(&self) -> &str {
        "HeaderWriterFilter"
    }
}
