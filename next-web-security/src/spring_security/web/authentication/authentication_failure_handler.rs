use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::core::authentication_error::AuthenticationError;

pub trait AuthenticationFailureHandler: Send + Sync {
    fn on_authentication_failure(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        error: &AuthenticationError,
    );
}
