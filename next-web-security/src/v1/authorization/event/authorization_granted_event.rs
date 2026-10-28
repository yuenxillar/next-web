use next_web_context::ApplicationEvent;
use next_web_core::BoxAny;
use std::{
    any::{Any, TypeId},
    ops::Deref,
    sync::Arc,
};

use crate::{
    authorization::{
        authorization_result::AuthorizationResult, event::authorization_event::AuthorizationEvent,
    },
    core::Authentication,
};

/// Event published when authorization is granted.
#[derive(Clone)]
pub struct AuthorizationGrantedEvent {
    event: AuthorizationEvent,
}

impl AuthorizationGrantedEvent {
    pub fn new(
        authentication: Arc<dyn Authentication>,
        secured_object_description: BoxAny,
        result: Arc<dyn AuthorizationResult>,
    ) -> Self {
        Self {
            event: AuthorizationEvent::new(authentication, secured_object_description, result),
        }
    }

    pub fn authentication(&self) -> Arc<dyn Authentication> {
        self.event.authentication()
    }

    pub fn object(&self) -> &dyn Any {
        self.event.object()
    }
    pub fn authorization_result(&self) -> &dyn AuthorizationResult {
        self.event.authorization_result()
    }
}

impl Deref for AuthorizationGrantedEvent {
    type Target = AuthorizationEvent;
    fn deref(&self) -> &Self::Target {
        &self.event
    }
}
impl ApplicationEvent for AuthorizationGrantedEvent {
    fn timestamp(&self) -> u64 {
        self.event.timestamp()
    }
    fn source(&self) -> &dyn Any {
        self.event.source()
    }
    fn event_type(&self) -> TypeId {
        TypeId::of::<Self>()
    }
    fn source_type(&self) -> TypeId {
        self.event.source_type()
    }
}
