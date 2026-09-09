use std::sync::Arc;

use next_web_context::{ApplicationEvent, ApplicationEventPublisher};

use crate::{
    authentication::{
        authentication_event_publisher::AuthenticationEventPublisher,
        event::{
            AuthenticationFailureBadCredentialsEvent, AuthenticationFailureExpiredEvent,
            AuthenticationFailureProviderNotFoundEvent, AuthenticationFailureServiceErrorEvent,
            AuthenticationSuccessEvent,
        },
    },
    core::{Authentication, AuthenticationError, AuthenticationErrorKind},
};

/// The default strategy for publishing authentication events.
///
/// Maps well-known `AuthenticationErrorKind` values to authentication events and
/// publishes them through the `ApplicationEventPublisher`. If configured as a bean,
/// it picks up the `ApplicationEventPublisher` automatically; otherwise the
/// constructor that takes the publisher as an argument should be used.
///
/// The following kinds are mapped by default:
///
/// * `BadCredentials`, `UsernameNotFound`, `CredentialsNotFound` map to an
///   `AuthenticationFailureBadCredentialsEvent`.
/// * `AccountStatus` maps to an `AuthenticationFailureExpiredEvent`.
/// * `ProviderNotFound` maps to an `AuthenticationFailureProviderNotFoundEvent`.
/// * Any other kind maps to an `AuthenticationFailureServiceExceptionEvent`.
///
/// When no publisher has been configured, publishing is a no-op.
pub struct DefaultAuthenticationEventPublisher {
    application_event_publisher: Option<Arc<dyn ApplicationEventPublisher>>,
}

impl DefaultAuthenticationEventPublisher {
    /// Creates a new publisher without an `ApplicationEventPublisher`. Use
    /// [`set_application_event_publisher`](Self::set_application_event_publisher) or
    /// [`with_application_event_publisher`](Self::with_application_event_publisher) to
    /// supply one before publishing.
    pub fn new() -> Self {
        Self {
            application_event_publisher: None,
        }
    }

    /// Creates a new publisher with the given `ApplicationEventPublisher`.
    pub fn with_application_event_publisher(
        application_event_publisher: Arc<dyn ApplicationEventPublisher>,
    ) -> Self {
        Self {
            application_event_publisher: Some(application_event_publisher),
        }
    }

    /// Sets the `ApplicationEventPublisher` used to publish events.
    pub fn set_application_event_publisher(
        &mut self,
        application_event_publisher: Arc<dyn ApplicationEventPublisher>,
    ) {
        self.application_event_publisher = Some(application_event_publisher);
    }

    /// Creates the failure event that corresponds to the given error kind.
    fn failure_event(
        error: AuthenticationError,
        authentication: Arc<dyn Authentication>,
    ) -> Box<dyn ApplicationEvent> {
        match error.kind() {
            AuthenticationErrorKind::BadCredentials
            | AuthenticationErrorKind::UsernameNotFound
            | AuthenticationErrorKind::CredentialsNotFound => Box::new(
                AuthenticationFailureBadCredentialsEvent::new(authentication, error),
            ),
            AuthenticationErrorKind::AccountStatus => Box::new(
                AuthenticationFailureExpiredEvent::new(authentication, error),
            ),
            AuthenticationErrorKind::ProviderNotFound => Box::new(
                AuthenticationFailureProviderNotFoundEvent::new(authentication, error),
            ),
            _ => Box::new(AuthenticationFailureServiceErrorEvent::new(
                authentication,
                error,
            )),
        }
    }
}

impl Default for DefaultAuthenticationEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthenticationEventPublisher for DefaultAuthenticationEventPublisher {
    fn publish_authentication_success(&self, authentication: Arc<dyn Authentication>) {
        if let Some(ref publisher) = self.application_event_publisher {
            let event: Box<dyn ApplicationEvent> =
                Box::new(AuthenticationSuccessEvent::new(authentication));
            let _ = publisher.publish_event(event);
        }
    }

    fn publish_authentication_failure(
        &self,
        error: AuthenticationError,
        authentication: Arc<dyn Authentication>,
    ) {
        if let Some(ref publisher) = self.application_event_publisher {
            let event = Self::failure_event(error, authentication);
            let _ = publisher.publish_event(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        any::TypeId,
        error::Error,
        sync::{Arc, Mutex},
    };

    use next_web_context::{ApplicationEvent, ApplicationEventPublisher};

    use super::*;
    use crate::{
        authentication::UsernamePasswordAuthenticationToken, web::authentication::AuthPrincipal,
    };

    #[derive(Default)]
    struct RecordingPublisher {
        events: Mutex<Vec<TypeId>>,
    }

    impl ApplicationEventPublisher for RecordingPublisher {
        fn publish_event(
            &self,
            event: Box<dyn ApplicationEvent>,
        ) -> Result<(), Box<dyn Error + Send + Sync>> {
            self.events.lock().unwrap().push(event.event_type());
            Ok(())
        }
    }

    fn authentication() -> Arc<dyn Authentication> {
        let principal: AuthPrincipal = Arc::new("user".to_owned());
        Arc::new(UsernamePasswordAuthenticationToken::unauthenticated(
            Some(principal),
            None,
        ))
    }

    #[test]
    fn does_not_publish_without_publisher() {
        let publisher = DefaultAuthenticationEventPublisher::new();
        publisher.publish_authentication_success(authentication());
        publisher.publish_authentication_failure(
            AuthenticationError::with_kind(
                "Bad credentials",
                AuthenticationErrorKind::BadCredentials,
            ),
            authentication(),
        );
    }

    #[test]
    fn publishes_success_event() {
        let publisher = Arc::new(RecordingPublisher::default());
        let event_publisher = DefaultAuthenticationEventPublisher::with_application_event_publisher(
            publisher.clone(),
        );
        event_publisher.publish_authentication_success(authentication());

        let types = publisher.events.lock().unwrap();
        assert_eq!(*types, vec![TypeId::of::<AuthenticationSuccessEvent>()]);
    }

    #[test]
    fn publishes_bad_credentials_event_for_bad_credentials_kind() {
        let publisher = Arc::new(RecordingPublisher::default());
        let event_publisher = DefaultAuthenticationEventPublisher::with_application_event_publisher(
            publisher.clone(),
        );
        event_publisher.publish_authentication_failure(
            AuthenticationError::with_kind(
                "Bad credentials",
                AuthenticationErrorKind::BadCredentials,
            ),
            authentication(),
        );

        let types = publisher.events.lock().unwrap();
        assert_eq!(
            *types,
            vec![TypeId::of::<AuthenticationFailureBadCredentialsEvent>()]
        );
    }

    #[test]
    fn publishes_provider_not_found_event_for_provider_not_found_kind() {
        let publisher = Arc::new(RecordingPublisher::default());
        let event_publisher = DefaultAuthenticationEventPublisher::with_application_event_publisher(
            publisher.clone(),
        );
        event_publisher.publish_authentication_failure(
            AuthenticationError::with_kind(
                "No provider",
                AuthenticationErrorKind::ProviderNotFound,
            ),
            authentication(),
        );

        let types = publisher.events.lock().unwrap();
        assert_eq!(
            *types,
            vec![TypeId::of::<AuthenticationFailureProviderNotFoundEvent>()]
        );
    }

    #[test]
    fn falls_back_to_service_exception_event_for_unmapped_kinds() {
        let publisher = Arc::new(RecordingPublisher::default());
        let event_publisher = DefaultAuthenticationEventPublisher::with_application_event_publisher(
            publisher.clone(),
        );
        event_publisher.publish_authentication_failure(
            AuthenticationError::with_kind("Boom", AuthenticationErrorKind::General),
            authentication(),
        );

        let types = publisher.events.lock().unwrap();
        assert_eq!(
            *types,
            vec![TypeId::of::<AuthenticationFailureServiceErrorEvent>()]
        );
    }
}
