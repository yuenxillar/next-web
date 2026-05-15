use crate::authentication::authentication_events::{
    AuthenticationFailureEvent, AuthenticationSuccessEvent,
};

pub trait AuthenticationEventPublisher: Send + Sync {
    fn publish_authentication_success(&self, _event: AuthenticationSuccessEvent) {}

    fn publish_authentication_failure(&self, _event: AuthenticationFailureEvent) {}
}

#[derive(Clone, Default)]
pub struct NullAuthenticationEventPublisher;

impl AuthenticationEventPublisher for NullAuthenticationEventPublisher {}
