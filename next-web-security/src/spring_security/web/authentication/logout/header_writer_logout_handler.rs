use std::sync::Arc;

use next_web_core::{
    async_trait,
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{
    core::Authentication,
    web::{authentication::logout::LogoutSuccessHandler, header::HeaderWriter},
};

#[derive(Clone)]
pub struct HeaderWriterLogoutHandler {
    header_writer: Arc<dyn HeaderWriter>,
}

impl HeaderWriterLogoutHandler {
    pub fn new<T>(header_writer: T) -> Self
    where
        T: HeaderWriter,
        T: 'static,
    {
        Self {
            header_writer: Arc::new(header_writer),
        }
    }
}

#[async_trait]
impl LogoutSuccessHandler for HeaderWriterLogoutHandler {
    async fn on_logout_success(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _authentication: Option<&Arc<dyn Authentication>>,
    ) -> Result<(), BoxError> {
        self.header_writer.write_headers(request, response);

        Ok(())
    }
}
