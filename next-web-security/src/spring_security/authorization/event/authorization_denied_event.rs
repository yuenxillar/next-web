use std::sync::Arc;

use crate::{
    authorization::{authorization_result::AuthorizationResult, event::authorization_event::AuthorizationEvent},
    core::authentication::Authentication,
};

/// Event published when authorization is denied.
#[derive(Clone)]
pub struct AuthorizationDeniedEvent {
    event: AuthorizationEvent,
}

impl AuthorizationDeniedEvent {
    pub fn new(
        authentication: Arc<dyn Authentication>,
        secured_object_description: impl Into<String>,
        result: Arc<dyn AuthorizationResult>,
    ) -> Self {
        Self {
            event: AuthorizationEvent::new(authentication, secured_object_description, result),
        }
    }

    pub fn authentication(&self) -> Arc<dyn Authentication> {
        self.event.authentication()
    }

    pub fn secured_object_description(&self) -> &str {
        self.event.secured_object_description()
    }

    pub fn authorization_result(&self) -> Arc<dyn AuthorizationResult> {
        self.event.authorization_result()
    }
}
