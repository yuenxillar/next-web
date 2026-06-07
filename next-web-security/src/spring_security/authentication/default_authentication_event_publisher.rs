use std::{collections::HashMap, sync::Arc};

use crate::{
    authentication::{
        authentication_event_publisher::AuthenticationEventPublisher,
        authentication_events::{AuthenticationFailureEvent, AuthenticationSuccessEvent},
    },
    core::{
        Authentication,
        authentication_error::{AuthenticationError, AuthenticationErrorKind},
    },
};

pub trait ApplicationEventPublisher: Send + Sync {
    fn publish_event(&self, event: Box<dyn std::any::Any + Send + Sync>);
}

pub struct DefaultAuthenticationEventPublisher {
    exception_mappings: HashMap<AuthenticationErrorKind, &'static str>,
    application_event_publisher: Option<Arc<dyn ApplicationEventPublisher>>,
}

impl DefaultAuthenticationEventPublisher {
    pub fn new() -> Self {
        let mut publisher = Self {
            exception_mappings: HashMap::new(),
            application_event_publisher: None,
        };
        publisher.register_default_mappings();
        publisher
    }

    pub fn set_application_event_publisher(
        &mut self,
        publisher: Arc<dyn ApplicationEventPublisher>,
    ) {
        self.application_event_publisher = Some(publisher);
    }

    fn register_default_mappings(&mut self) {
        self.exception_mappings
            .insert(AuthenticationErrorKind::BadCredentials, "BadCredentials");
        self.exception_mappings
            .insert(AuthenticationErrorKind::AccountStatus, "AccountExpired");
        self.exception_mappings
            .insert(AuthenticationErrorKind::CredentialsNotFound, "BadCredentials");
        self.exception_mappings
            .insert(AuthenticationErrorKind::ProviderNotFound, "ProviderNotFound");
        self.exception_mappings
            .insert(AuthenticationErrorKind::Service, "ServiceException");
        self.exception_mappings
            .insert(AuthenticationErrorKind::InternalService, "ServiceException");
    }

    fn event_name_for(&self, kind: AuthenticationErrorKind) -> &str {
        self.exception_mappings
            .get(&kind)
            .copied()
            .unwrap_or("Unknown")
    }
}

impl Default for DefaultAuthenticationEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthenticationEventPublisher for DefaultAuthenticationEventPublisher {
    fn publish_authentication_success(&self, event: AuthenticationSuccessEvent) {
        if let Some(ref publisher) = self.application_event_publisher {
            publisher.publish_event(Box::new(event));
        }
    }

    fn publish_authentication_failure(&self, event: AuthenticationFailureEvent) {
        if let Some(ref publisher) = self.application_event_publisher {
            let _ = self.event_name_for(event.error().kind());
            publisher.publish_event(Box::new(event));
        }
    }
}
