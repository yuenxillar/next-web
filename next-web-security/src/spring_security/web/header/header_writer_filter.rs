use std::sync::Arc;

use next_web_core::traits::{
    filter::HttpFilterChain,
    http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::web::header::HeaderWriter;

pub struct HeaderWriterFilter {
    /// The HeaderWriter to write headers to the response. See CompositeHeaderWriter
    header_writers: Vec<Arc<dyn HeaderWriter>>,

    should_write_headers_eagerly: bool,
}

impl HeaderWriterFilter {
    pub fn new(header_writers: Vec<Arc<dyn HeaderWriter>>) -> Self {
        Self {
            header_writers,
            should_write_headers_eagerly: false,
        }
    }

    pub fn add_header_writer(&mut self, header_writer: Arc<dyn HeaderWriter>) {
        self.header_writers.push(header_writer);
    }

    pub fn clear_header_writers(&mut self) {
        self.header_writers.clear();
    }
}

impl HeaderWriterFilter {
    async fn do_headers_before(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &mut dyn HttpFilterChain,
    ) {
        self.write_headers(request, response);
        filter_chain.do_filter(request, response).await;
    }

    async fn do_headers_after(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &mut dyn HttpFilterChain,
    ) {
        filter_chain.do_filter(request, response).await;
    }

    fn write_headers(&self, request: &mut dyn HttpRequest, response: &mut dyn HttpResponse) {
        self.header_writers.iter().for_each(|writer| {
            writer.write_headers(request, response);
        });
    }
}
