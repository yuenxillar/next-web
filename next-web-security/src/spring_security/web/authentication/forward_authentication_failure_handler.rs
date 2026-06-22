use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::web::authentication::authentication_failure_handler::AuthenticationFailureHandler;

#[derive(Clone)]
pub struct ForwardAuthenticationFailureHandler {
    pub(crate) forward_url: Box<str>,
}

impl ForwardAuthenticationFailureHandler {
    pub fn new(forward_url: impl Into<Box<str>>) -> Self {
        let forward_url = forward_url.into();
        assert!(
            forward_url.starts_with("/"),
            "{} is not a valid forward URL",
            forward_url.as_ref()
        );
        Self { forward_url }
    }
}

impl AuthenticationFailureHandler for ForwardAuthenticationFailureHandler {
    fn on_authentication_failure(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        error: &crate::core::authentication_error::AuthenticationError,
    ) -> Result<(), BoxError> {
        request.set_attribute("NEXT_SECURITY_LAST_ERROR", error.clone().into());
        // TODO: request_dispatcher always returns None currently;
        // forward will be implemented when RequestDispatcher is wired up.
        let _ = response;

        todo!()
    }
}
