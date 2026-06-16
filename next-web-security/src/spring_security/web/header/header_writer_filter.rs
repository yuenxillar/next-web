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

/// Filter implementation to add headers to the current response. Can be useful to add
/// certain headers which enable browser protection. Like X-Frame-Options, X-XSS-Protection
/// and X-Content-Type-Options.
#[derive(Clone)]
pub struct HeaderWriterFilter {
    /// The `HeaderWriter`s to write headers to the response.
    header_writers: Vec<Arc<dyn HeaderWriter>>,

    /// Indicates whether to write the headers at the beginning of the request.
    should_write_headers_eagerly: bool,
}

impl HeaderWriterFilter {
    pub fn new(header_writers: Vec<Arc<dyn HeaderWriter>>) -> Self {
        Self {
            header_writers,
            should_write_headers_eagerly: false,
        }
    }

    pub fn set_should_write_headers_eagerly(&mut self, should_write_headers_eagerly: bool) {
        self.should_write_headers_eagerly = should_write_headers_eagerly;
    }

    fn write_headers(&self, request: &dyn HttpRequest, response: &mut dyn HttpResponse) {
        for writer in self.header_writers.iter() {
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
        if self.should_write_headers_eagerly {
            self.write_headers(request, response);
            filter_chain.do_filter(request, response).await
        } else {
            match filter_chain.do_filter(request, response).await {
                Ok(_) => {
                    self.write_headers(request, response);
                    Ok(())
                }
                Err(err) => {
                    self.write_headers(request, response);
                    Err(err)
                }
            }
        }
    }
}

impl Named for HeaderWriterFilter {
    fn name(&self) -> &str {
        "HeaderWriterFilter"
    }
}
