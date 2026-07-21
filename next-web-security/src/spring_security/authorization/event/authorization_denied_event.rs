use std::sync::Arc;

use next_web_context::ApplicationEvent;
use next_web_core::BoxAny;

use crate::{
    authorization::{
        authorization_result::AuthorizationResult, event::authorization_event::AuthorizationEvent,
    },
    core::Authentication,
};

/// Event published when authorization is denied.
#[derive(Clone)]
pub struct AuthorizationDeniedEvent {
    inner: AuthorizationEvent,
}

impl AuthorizationDeniedEvent {
    pub fn new(
        authentication: Arc<dyn Authentication>,
        object: BoxAny,
        result: Box<dyn AuthorizationResult>,
    ) -> Self {
        Self {
            inner: AuthorizationEvent::new(authentication, object, result),
        }
    }

    pub fn authentication(&self) -> Arc<dyn Authentication> {
        self.inner.authentication()
    }

    pub fn authorization_result(&self) -> &dyn AuthorizationResult {
        self.inner.authorization_result()
    }
}

impl ApplicationEvent for AuthorizationDeniedEvent {
    fn timestamp(&self) -> u64 {
        self.inner.timestamp()
    }

    fn source(&self) -> &dyn std::any::Any {
        self.inner.source()
    }
}
