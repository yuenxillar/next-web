use std::{
    any::{Any, TypeId},
    ops::Deref,
    sync::Arc,
};

use next_web_context::ApplicationEvent;

use crate::{authentication::event::BaseAuthenticationEvent, core::Authentication};

/// Indicates that an authentication request completed successfully.
#[derive(Clone)]
pub struct AuthenticationSuccessEvent {
    base: BaseAuthenticationEvent,
}

impl AuthenticationSuccessEvent {
    pub fn new(authentication: Arc<dyn Authentication>) -> Self {
        Self {
            base: BaseAuthenticationEvent::new(authentication),
        }
    }
}

impl Deref for AuthenticationSuccessEvent {
    type Target = BaseAuthenticationEvent;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl ApplicationEvent for AuthenticationSuccessEvent {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        authentication::UsernamePasswordAuthenticationToken, web::authentication::AuthPrincipal,
    };

    #[test]
    fn exposes_authentication_and_event_metadata() {
        let principal: AuthPrincipal = Arc::new("user".to_owned());
        let authentication: Arc<dyn Authentication> = Arc::new(
            UsernamePasswordAuthenticationToken::unauthenticated(Some(principal), None),
        );
        let event = AuthenticationSuccessEvent::new(authentication.clone());

        assert!(Arc::ptr_eq(event.authentication(), &authentication));
        assert_eq!(
            event.event_type(),
            TypeId::of::<AuthenticationSuccessEvent>()
        );
        assert_eq!(event.source_type(), TypeId::of::<Arc<dyn Authentication>>());
        assert!(event.timestamp() > 0);
    }
}
