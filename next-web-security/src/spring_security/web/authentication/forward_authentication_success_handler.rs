use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

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
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
        _authentication: &dyn crate::core::Authentication,
    ) {
        // TODO: request_dispatcher always returns None currently;
        // forward will be implemented when RequestDispatcher is wired up.
        let _ = (req, resp);
    }
}
