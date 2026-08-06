use next_web_core::{
    error::BoxError,
    http::StatusCode,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{core::authentication_error::AuthenticationError, web::AuthenticationEntryPoint};

/// An AuthenticationEntryPoint that sends a generic HttpStatus as a response. Useful for JavaScript
/// clients which cannot use Basic authentication since the browser intercepts the response
#[derive(Clone)]
pub struct HttpStatusEntryPoint {
    status_code: StatusCode,
}

impl HttpStatusEntryPoint {
    pub fn new(status_code: StatusCode) -> Self {
        Self { status_code }
    }
}

impl AuthenticationEntryPoint for HttpStatusEntryPoint {
    fn commence(
        &self,
        _request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _auth_error: &AuthenticationError,
    ) -> Result<(), BoxError> {
        response.set_status_code(self.status_code);

        Ok(())
    }
}
