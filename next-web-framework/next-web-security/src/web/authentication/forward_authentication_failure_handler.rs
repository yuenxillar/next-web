use next_web_core::{anys::any_map::AnyMap, traits::http::http_request::HttpRequest};

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
        request: &axum::extract::Request,
        response: &mut axum::response::Response,
        error: &crate::core::authentication_error::AuthenticationError,
    ) {
        if let Some(any_map) = request.extensions().get::<AnyMap>() {
            any_map.set("NEXT_SECURITY_LAST_ERROR".to_string(), error.clone().into());
            request
                .get_request_dispatcher(&self.forward_url)
                .map(|dispatcher| dispatcher.forward(request, response));
        };
    }
}
