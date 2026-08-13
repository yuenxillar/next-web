use std::sync::Arc;

use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::{
    access::AccessDeniedError,
    web::{access::AccessDeniedHandler, session::InvalidSessionStrategy},
};

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

impl AccessDeniedHandler for InvalidSessionAccessDeniedHandler {
    fn handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        access_denied_error: &AccessDeniedError,
    ) -> Result<(), next_web_core::error::BoxError> {
        todo!()
    }
}
