use std::sync::Arc;

use next_web_core::http::StatusCode;
use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use tracing::debug;

use crate::{
    core::authentication_error::AuthenticationError,
    web::{
        authentication::authentication_failure_handler::AuthenticationFailureHandler,
        redirect_strategy::{DefaultRedirectStrategy, RedirectStrategy},
    },
};

#[derive(Clone)]
pub struct SimpleUrlAuthenticationFailureHandler {
    default_failure_url: Option<Box<str>>,
    forward_to_destination: bool,
    allow_session_creation: bool,
    redirect_strategy: Arc<dyn RedirectStrategy>,
}

impl SimpleUrlAuthenticationFailureHandler {
    pub fn new(default_failure_url: &str) -> Self {
        assert!(
            !default_failure_url.is_empty() && default_failure_url.starts_with("/"),
            "{}  is not a valid redirect URL",
            default_failure_url
        );

        Self {
            default_failure_url: Some(default_failure_url.into()),
            forward_to_destination: false,
            allow_session_creation: true,
            redirect_strategy: Arc::new(DefaultRedirectStrategy::default()),
        }
    }

    pub async fn save_error(&self, request: &mut dyn HttpRequest, error: &AuthenticationError) {
        if self.forward_to_destination {
            request.set_attribute("NEXT_SECURITY_LAST_ERROR", error.clone().into());
        } else {
            let session = request.session(false);
            if session.is_some() || self.allow_session_creation {
                // Set Error in session
                // session.unwrap().set("NEXT_SECURITY_LAST_ERROR", error.clone().into_boxed());
            }
        }
    }

    pub fn set_default_failure_url(&mut self, default_failure_url: &str) {
        assert!(
            !default_failure_url.is_empty() && default_failure_url.starts_with("/"),
            "{}  is not a valid redirect URL",
            default_failure_url
        );
        self.default_failure_url = Some(default_failure_url.into());
    }

    pub fn is_use_forward(&self) -> bool {
        self.forward_to_destination
    }

    pub fn set_use_forward(&mut self, forward_to_destination: bool) {
        self.forward_to_destination = forward_to_destination
    }

    pub fn set_redirect_strategy(&mut self, redirect_strategy: Arc<dyn RedirectStrategy>) {
        self.redirect_strategy = redirect_strategy;
    }

    pub fn get_redirect_strategy(&self) -> &dyn RedirectStrategy {
        self.redirect_strategy.as_ref()
    }

    pub fn is_allow_session_creation(&self) -> bool {
        self.allow_session_creation
    }

    pub fn set_allow_session_creation(&mut self, allow_session_creation: bool) {
        self.allow_session_creation = allow_session_creation;
    }
}

impl AuthenticationFailureHandler for SimpleUrlAuthenticationFailureHandler {
    fn on_authentication_failure(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        error: &AuthenticationError,
    ) -> Result<(), BoxError> {
        if self.default_failure_url.is_none() {
            debug!("Sending 401 Unauthorized error");

            response.set_status_code(StatusCode::UNAUTHORIZED);
            response.set_body(StatusCode::UNAUTHORIZED.as_str().to_string().into_bytes());
        } else {
            if self.forward_to_destination {
                debug!(
                    "Forwarding to {}",
                    self.default_failure_url
                        .as_ref()
                        .map(|s| s.as_ref())
                        .unwrap_or_default()
                );
                // TODO: request.request_dispatcher always returns None currently;
                // forward to destination will be implemented when RequestDispatcher is wired up.
            } else {
                self.redirect_strategy.send_redirect(
                    request,
                    response,
                    self.default_failure_url
                        .as_ref()
                        .map(|s| s.as_ref())
                        .unwrap_or_default(),
                );
            }
        }

        todo!()
    }
}
