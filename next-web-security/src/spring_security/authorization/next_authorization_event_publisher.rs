use std::sync::Arc;

use next_web_context::ApplicationEventPublisher;
use next_web_core::BoxAny;

use crate::{
    authorization::{
        authorization_event_publisher::AuthorizationEventPublisher,
        authorization_result::AuthorizationResult,
        event::authorization_denied_event::AuthorizationDeniedEvent,
    },
    core::Authentication,
};

/// An implementation of AuthorizationEventPublisher that uses Spring's event publishing support.
/// Because AuthorizationGrantedEvents typically require additional business logic to decide whether to
/// publish, this implementation only publishes AuthorizationDeniedEvents.
pub struct NextAuthorizationEventPublisher {
    event_publisher: Arc<dyn ApplicationEventPublisher>,
    should_publish_result: Box<dyn Fn(&dyn AuthorizationResult) -> bool + Send + Sync>,
}

impl NextAuthorizationEventPublisher {
    /// Construct this publisher using ApplicationEventPublisher
    pub fn new(event_publisher: Arc<dyn ApplicationEventPublisher>) -> Self {
        Self {
            event_publisher,
            should_publish_result: Box::new(|result: &dyn AuthorizationResult| {
                !result.is_granted()
            }),
        }
    }

    /// Use this predicate to test whether to publish an event.
    pub fn set_should_publish_result<F>(&mut self, should_publish: F)
    where
        F: Fn(&dyn AuthorizationResult) -> bool + Send + Sync + 'static,
    {
        self.should_publish_result = Box::new(should_publish);
    }
}

impl AuthorizationEventPublisher for NextAuthorizationEventPublisher {
    /// Publish the given details in the form of an event, typically AuthorizationGrantedEvent or AuthorizationDeniedEvent.
    /// Note that success events can be very noisy if enabled by default.
    /// Because of this implementations may choose to drop success events by default.
    fn publish_authorization_event(
        &self,
        authentication: Arc<dyn Authentication>,
        object: BoxAny,
        result: Option<Arc<dyn AuthorizationResult>>,
    ) {
        let result = match result {
            Some(r) => r,
            None => return,
        };

        if !(self.should_publish_result)(result.as_ref()) {
            return;
        }

        let failure = AuthorizationDeniedEvent::new(authentication, object, result);
        self.event_publisher
            .publish_event(Box::new(failure))
            .inspect_err(|err| {
                tracing::error!("Failed to publish authorization event: {}", err);
            })
            .ok();
    }
}
