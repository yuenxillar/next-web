use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::web::{
    authentication::authentication_success_handler::AuthenticationSuccessHandler, util::UrlUtils,
};

/// Forward Authentication Success Handler
#[derive(Clone)]
pub struct ForwardAuthenticationSuccessHandler {
    pub(crate) forward_url: Box<str>,
}

impl ForwardAuthenticationSuccessHandler {
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

impl AuthenticationSuccessHandler for ForwardAuthenticationSuccessHandler {
    fn on_authentication_success(
        &self,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
        _authentication: &dyn crate::core::Authentication,
    ) {
        if let Some(dispatcher) = req.request_dispatcher(&self.forward_url) {
            dispatcher
                .forward(req, resp)
                .inspect_err(|err| {
                    eprintln!("Failed to forward authentication success: {}", err);
                })
                .ok();
        }
    }
}
