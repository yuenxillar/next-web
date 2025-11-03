use next_web_core::traits::http::http_request::HttpRequest;

use crate::web::authentication::authentication_success_handler::AuthenticationSuccessHandler;

#[derive(Clone)]
pub struct ForwardAuthenticationSuccessHandler {
    pub(crate) forward_url: Box<str>,
}

impl ForwardAuthenticationSuccessHandler {
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

impl AuthenticationSuccessHandler for ForwardAuthenticationSuccessHandler {
    fn on_authentication_success(
        &self,
        request: &axum::extract::Request,
        response: &mut axum::response::Response,
        _authentication: &dyn crate::core::authentication::Authentication,
    ) {
        if let Some(dispatcher) = request.request_dispatcher(&self.forward_url) {
            dispatcher.forward(request, response);
        }
    }
}
