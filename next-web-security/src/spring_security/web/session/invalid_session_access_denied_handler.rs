use std::sync::Arc;

use crate::web::{access::AccessDeniedHandler, session::InvalidSessionStrategy};

pub struct InvalidSessionAccessDeniedHandler {
    invalid_session_strategy: Arc<dyn InvalidSessionStrategy>,
}

impl InvalidSessionAccessDeniedHandler {
    pub fn new(invalid_session_strategy: Arc<dyn InvalidSessionStrategy>) -> Self {
        Self {
            invalid_session_strategy,
        }
    }
}


impl AccessDeniedHandler  for  InvalidSessionAccessDeniedHandler {
    fn handle(
        &self,
        request: &mut dyn next_web_core::traits::http::http_request::HttpRequest,
        response: &mut dyn next_web_core::traits::http::http_response::HttpResponse,
        access_denied_error: crate::web::access::AccessDeniedError,
    ) -> Result<(), next_web_core::error::BoxError> {
        todo!()
    }
}