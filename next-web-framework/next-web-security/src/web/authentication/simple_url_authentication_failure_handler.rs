use std::sync::Arc;

use axum::{extract::Request, http::StatusCode, response::Response};
use next_web_core::{
    anys::{any_map::AnyMap, any_value::AnyValue},
    convert::into_box::IntoBox,
    traits::http::http_request::HttpRequest,
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

    pub async fn save_error(&self, request: &Request, error: &AuthenticationError) {
        if self.forward_to_destination {
            match request.extensions().get::<AnyMap>() {
                Some(map) => {
                    map.set(
                        "NEXT_SECURITY_LAST_ERROR".to_string(),
                        AnyValue::Object(error.clone().into_boxed()),
                    ).await;
                }
                None => {}
            }
        } else {
            let session = request.get_session("sessionid");
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
        request: &Request,
        response: &mut Response,
        error: &AuthenticationError,
    ) {
        if self.default_failure_url.is_none() {
            debug!("Sending 401 Unauthorized error");

            *response.status_mut() = StatusCode::UNAUTHORIZED;
            *response.body_mut() = StatusCode::UNAUTHORIZED.as_str().to_string().into();
        } else {
            // TODO: Save error in session
            // self.save_error(request, error).await;

            if self.forward_to_destination {
                debug!(
                    "Forwarding to {}",
                    self.default_failure_url
                        .as_ref()
                        .map(|s| s.as_ref())
                        .unwrap_or_default()
                );
                if let Some(dispatcher) = request.get_request_dispatcher(
                    self.default_failure_url
                        .as_ref()
                        .map(|s| s.as_ref())
                        .unwrap_or_default(),
                ) {
                    dispatcher.forward(request, response).unwrap();
                }
            } else {
                self.redirect_strategy.send_redirect(
                    None,
                    self.default_failure_url
                        .as_ref()
                        .map(|s| s.as_ref())
                        .unwrap_or_default(),
                    response,
                );
            }
        }
    }
}
