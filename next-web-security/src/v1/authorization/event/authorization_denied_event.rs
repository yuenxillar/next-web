use std::{
    any::{Any, TypeId},
    ops::Deref,
    sync::Arc,
};

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
    base: AuthorizationEvent,
}

impl AuthorizationDeniedEvent {
    pub fn new(
        authentication: Arc<dyn Authentication>,
        object: BoxAny,
        result: Arc<dyn AuthorizationResult>,
    ) -> Self {
        Self {
            base: AuthorizationEvent::new(authentication, object, result),
        }
    }

    pub fn authentication(&self) -> Arc<dyn Authentication> {
        self.base.authentication()
    }

    pub fn authorization_result(&self) -> &dyn AuthorizationResult {
        self.base.authorization_result()
    }
}

impl Deref for AuthorizationDeniedEvent {
    type Target = AuthorizationEvent;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl ApplicationEvent for AuthorizationDeniedEvent {
    fn timestamp(&self) -> u64 {
        self.base.timestamp()
    }
    fn source(&self) -> &dyn Any {
        self.base.source()
    }
    fn event_type(&self) -> TypeId {
        TypeId::of::<Self>()
    }
    fn source_type(&self) -> TypeId {
        self.base.source_type()
    }
}
