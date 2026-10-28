use std::{
    any::{Any, TypeId},
    ops::Deref,
    sync::Arc,
};

use next_web_context::ApplicationEvent;

use crate::{
    authentication::event::BaseAuthenticationEvent,
    core::{Authentication, AuthenticationError},
};

/// Common state for an authentication failure event.
#[derive(Clone)]
pub struct BaseAuthenticationFailureEvent {
    base: BaseAuthenticationEvent,
    error: AuthenticationError,
}

impl BaseAuthenticationFailureEvent {
    pub fn new(authentication: Arc<dyn Authentication>, error: AuthenticationError) -> Self {
        Self {
            base: BaseAuthenticationEvent::new(authentication),
            error,
        }
    }

    pub fn error(&self) -> &AuthenticationError {
        &self.error
    }
}

impl Deref for BaseAuthenticationFailureEvent {
    type Target = BaseAuthenticationEvent;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

macro_rules! failure_event {
    ($name:ident) => {
        #[derive(Clone)]
        pub struct $name {
            base: BaseAuthenticationFailureEvent,
        }

        impl $name {
            pub fn new(
                authentication: Arc<dyn Authentication>,
                error: AuthenticationError,
            ) -> Self {
                Self {
                    base: BaseAuthenticationFailureEvent::new(authentication, error),
                }
            }
        }

        impl Deref for $name {
            type Target = BaseAuthenticationFailureEvent;
            fn deref(&self) -> &Self::Target {
                &self.base
            }
        }

        impl ApplicationEvent for $name {
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
    };
}

failure_event!(AuthenticationFailureBadCredentialsEvent);
failure_event!(AuthenticationFailureCredentialsExpiredEvent);
failure_event!(AuthenticationFailureDisabledEvent);
failure_event!(AuthenticationFailureExpiredEvent);
failure_event!(AuthenticationFailureLockedEvent);
failure_event!(AuthenticationFailureProviderNotFoundEvent);
failure_event!(AuthenticationFailureProxyUntrustedEvent);
failure_event!(AuthenticationFailureServiceErrorEvent);
