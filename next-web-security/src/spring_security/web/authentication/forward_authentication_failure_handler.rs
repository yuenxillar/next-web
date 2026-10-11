use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::web::{
    authentication::authentication_failure_handler::AuthenticationFailureHandler, util::UrlUtils,
    WebAttributes,
};

/// Forward Authentication Failure Handler
#[derive(Clone, Debug)]
pub struct ForwardAuthenticationFailureHandler {
    pub(crate) forward_url: Box<str>,
}

impl ForwardAuthenticationFailureHandler {
    pub fn new(forward_url: impl Into<Box<str>>) -> Self {
        let forward_url = forward_url.into();
        assert!(
            UrlUtils::is_valid_redirect_url(forward_url.as_ref()),
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
        error: &crate::core::AuthenticationError,
    ) -> Result<(), BoxError> {
        request.set_attribute(WebAttributes::AUTHENTICATION_ERROR, error.clone().into());
        if let Some(resource_dispatcher) = request.resource_dispatcher() {
            return resource_dispatcher.forward(&self.forward_url, request, response);
        }

        Ok(())
    }
}
