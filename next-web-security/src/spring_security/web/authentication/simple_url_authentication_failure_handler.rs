use std::fmt::Debug;
use std::sync::Arc;

use next_web_core::http::StatusCode;
use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use tracing::{debug, enabled, trace, Level};

use crate::web::util::UrlUtils;
use crate::web::WebAttributes;
use crate::{
    core::AuthenticationError,
    web::{
        authentication::authentication_failure_handler::AuthenticationFailureHandler,
        redirect_strategy::{DefaultRedirectStrategy, RedirectStrategy},
    },
};

/// AuthenticationFailureHandler which performs a redirect to the value of the defaultFailureUrl
/// property when the onAuthenticationFailure method is called. If the property has not been set it will
/// send a 401 response to the client, with the error message from the AuthenticationException which caused the failure.
///
/// If the useForward property is set, a RequestDispatcher.forward call will be made to the destination
/// instead of a redirect.
#[derive(Clone)]
pub struct SimpleUrlAuthenticationFailureHandler {
    default_failure_url: Option<Box<str>>,
    forward_to_destination: bool,
    allow_session_creation: bool,
    redirect_strategy: Arc<dyn RedirectStrategy>,
}

impl SimpleUrlAuthenticationFailureHandler {
    pub fn new(default_failure_url: &str) -> Self {
        let mut handler = Self::default();
        handler.set_default_failure_url(default_failure_url);

        handler
    }

    /// Caches the `AuthenticationError` for use in view rendering.
    ///
    /// If `forward_to_destination` is set to true, request scope will be used,
    /// otherwise it will attempt to store the exception in the session. If there is no
    /// session and `allow_session_creation` is `true` a session will be created.
    /// Otherwise the exception will not be stored.
    pub fn save_error(&self, request: &mut dyn HttpRequest, error: &AuthenticationError) {
        let error = error.clone().into();
        if self.forward_to_destination {
            request.set_attribute(WebAttributes::AUTHENTICATION_ERROR, error);
        } else {
            if let Some(session) = request.session() {
                if self.allow_session_creation {
                    session.set_attribute(WebAttributes::AUTHENTICATION_ERROR, error);
                }
            }
        }
    }

    /// The URL which will be used as the failure destination.
    ///
    /// # Arguments
    /// * `default_failure_url` - the failure URL, for example "/loginFailed.jsp"
    pub fn set_default_failure_url(&mut self, default_failure_url: &str) {
        assert!(
            UrlUtils::is_valid_redirect_url(default_failure_url),
            "{}  is not a valid redirect URL",
            default_failure_url
        );
        self.default_failure_url = Some(default_failure_url.into());
    }

    pub fn is_use_forward(&self) -> bool {
        self.forward_to_destination
    }

    /// If set to true, performs a forward to the failure destination URL instead of a redirect. Defaults to false.
    pub fn set_use_forward(&mut self, forward_to_destination: bool) {
        self.forward_to_destination = forward_to_destination
    }

    /// Allows overriding of the behaviour when redirecting to a target URL.
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
    /// Performs the redirect or forward to the defaultFailureUrl if set, otherwise returns a 401 error code.
    /// If redirecting or forwarding, saveException will be called to cache the exception for use in the target view.
    fn on_authentication_failure(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        error: &AuthenticationError,
    ) -> Result<(), BoxError> {
        if self.default_failure_url.is_none() {
            if enabled!(Level::TRACE) {
                trace!("No default failure URL set, sending 401 Unauthorized error");
            } else {
                debug!("Sending 401 Unauthorized error");
            }

            response.set_status_code(StatusCode::UNAUTHORIZED);
            response.set_body(b"Unauthorized".to_vec());
        } else {
            self.save_error(request, error);
            if self.forward_to_destination {
                if let Some(default_failure_url) = self.default_failure_url.as_deref() {
                    debug!("Forwarding to {}", default_failure_url);
                    if let Some(request_dispatcher) =
                        request.request_dispatcher(default_failure_url)
                    {
                        request_dispatcher.forward(request, response)?;
                    }
                }
            } else {
                self.redirect_strategy.send_redirect(
                    request,
                    response,
                    self.default_failure_url.as_deref().unwrap_or_default(),
                )?;
            }
        }

        Ok(())
    }
}

impl Debug for SimpleUrlAuthenticationFailureHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimpleUrlAuthenticationFailureHandler")
            .field("default_failure_url", &self.default_failure_url)
            .field("forward_to_destination", &self.forward_to_destination)
            .field("allow_session_creation", &self.allow_session_creation)
            .field("redirect_strategy", &"none")
            .finish()
    }
}

impl Default for SimpleUrlAuthenticationFailureHandler {
    fn default() -> Self {
        Self {
            default_failure_url: None,
            forward_to_destination: false,
            allow_session_creation: true,
            redirect_strategy: Arc::new(DefaultRedirectStrategy::default()),
        }
    }
}
