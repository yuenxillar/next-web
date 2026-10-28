use std::{
    any::{Any, TypeId},
    sync::Arc,
};

use next_web_context::{ApplicationEvent, EventAttributes};

use crate::core::{Authentication, AuthenticationError};

/// Application event which indicates successful authentication.
#[derive(Clone)]
pub struct AuthenticationSuccessEvent {
    base: EventAttributes<Arc<dyn Authentication>>,
}

impl AuthenticationSuccessEvent {
    pub fn new(authentication: Arc<dyn Authentication>) -> Self {
        Self {
            base: EventAttributes::new(authentication),
        }
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

/// Base application event which indicates authentication failure for some reason.
#[derive(Clone)]
pub struct BaseAuthenticationFailureEvent {
    error: AuthenticationError,

    event_attributes: EventAttributes<Arc<dyn Authentication>>,
}

impl BaseAuthenticationFailureEvent {
    pub fn new(authentication: &Arc<dyn Authentication>, error: AuthenticationError) -> Self {
        Self {
            error,
            event_attributes: EventAttributes::new(Arc::clone(authentication)),
        }
    }

    pub fn error(&self) -> &AuthenticationError {
        &self.error
    }
}

/// Application event which indicates successful logout
#[derive(Clone)]
pub struct LogoutSuccessEvent {
    base: EventAttributes<Arc<dyn Authentication>>,
}

impl LogoutSuccessEvent {
    pub fn new(authentication: Arc<dyn Authentication>) -> Self {
        Self {
            base: EventAttributes::new(authentication),
        }
    }
}

impl ApplicationEvent for LogoutSuccessEvent {
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

macro_rules! failure_event_wrapper {
    ($name:ident) => {
        #[derive(Clone)]
        pub struct $name {
            base_event: BaseAuthenticationFailureEvent,
        }

        impl $name {
            pub fn new(
                authentication: &Arc<dyn Authentication>,
                error: AuthenticationError,
            ) -> Self {
                Self {
                    base_event: BaseAuthenticationFailureEvent::new(authentication, error),
                }
            }
        }

        impl ApplicationEvent for $name {
            fn timestamp(&self) -> u64 {
                self.base_event.event_attributes.timestamp()
            }

            fn source(&self) -> &dyn Any {
                self.base_event.event_attributes.source()
            }

            fn event_type(&self) -> TypeId {
                TypeId::of::<$name>()
            }

            fn source_type(&self) -> TypeId {
                self.base_event.event_attributes.source_type()
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
failure_event_wrapper!(AuthenticationFailureServiceErrorEvent);
