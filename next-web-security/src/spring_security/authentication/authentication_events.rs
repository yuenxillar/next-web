use std::sync::Arc;

use crate::core::{Authentication, authentication_error::AuthenticationError};

#[derive(Clone)]
pub struct AuthenticationSuccessEvent {
    authentication: Arc<dyn Authentication>,
}

impl AuthenticationSuccessEvent {
    pub fn new(authentication: Arc<dyn Authentication>) -> Self {
        Self { authentication }
    }

    pub fn authentication(&self) -> Arc<dyn Authentication> {
        self.authentication.clone()
    }
}

#[derive(Clone)]
pub struct AuthenticationFailureEvent {
    authentication_type: String,
    error: AuthenticationError,
}

impl AuthenticationFailureEvent {
    pub fn new(authentication: &dyn Authentication, error: AuthenticationError) -> Self {
        Self {
            authentication_type: authentication.authentication_type().to_string(),
            error,
        }
    }

    pub fn authentication_type(&self) -> &str {
        &self.authentication_type
    }

    pub fn error(&self) -> &AuthenticationError {
        &self.error
    }
}

#[derive(Clone)]
pub struct InteractiveAuthenticationSuccessEvent {
    authentication: Arc<dyn Authentication>,
}

impl InteractiveAuthenticationSuccessEvent {
    pub fn new(authentication: Arc<dyn Authentication>) -> Self {
        Self { authentication }
    }

    pub fn authentication(&self) -> Arc<dyn Authentication> {
        self.authentication.clone()
    }
}

#[derive(Clone)]
pub struct LogoutSuccessEvent {
    authentication: Arc<dyn Authentication>,
}

impl LogoutSuccessEvent {
    pub fn new(authentication: Arc<dyn Authentication>) -> Self {
        Self { authentication }
    }

    pub fn authentication(&self) -> Arc<dyn Authentication> {
        self.authentication.clone()
    }
}

macro_rules! failure_event_wrapper {
    ($name:ident) => {
        #[derive(Clone)]
        pub struct $name {
            event: AuthenticationFailureEvent,
        }

        impl $name {
            pub fn new(authentication: &dyn Authentication, error: AuthenticationError) -> Self {
                Self {
                    event: AuthenticationFailureEvent::new(authentication, error),
                }
            }

            pub fn event(&self) -> &AuthenticationFailureEvent {
                &self.event
            }
        }
    };
}

failure_event_wrapper!(AuthenticationFailureBadCredentialsEvent);
failure_event_wrapper!(AuthenticationFailureCredentialsExpiredEvent);
failure_event_wrapper!(AuthenticationFailureDisabledEvent);
failure_event_wrapper!(AuthenticationFailureExpiredEvent);
failure_event_wrapper!(AuthenticationFailureLockedEvent);
failure_event_wrapper!(AuthenticationFailureProviderNotFoundEvent);
failure_event_wrapper!(AuthenticationFailureServiceExceptionEvent);
