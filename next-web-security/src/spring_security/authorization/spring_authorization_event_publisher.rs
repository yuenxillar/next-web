use std::sync::Arc;

use crate::{
    authorization::{
        authorization_event_publisher::AuthorizationEventPublisher,
        authorization_result::AuthorizationResult,
        event::authorization_denied_event::AuthorizationDeniedEvent,
    },
    core::Authentication,
};

/// A publisher that filters to only publish denied events by default
/// (to avoid noisy success events).
pub struct SpringAuthorizationEventPublisher {
    event_publisher: Arc<dyn ApplicationEventPublisher>,
    should_publish: Box<dyn Fn(&dyn AuthorizationResult) -> bool + Send + Sync>,
}

pub trait ApplicationEventPublisher: Send + Sync {
    fn publish_event(&self, event: Box<dyn std::any::Any + Send + Sync>);
}

impl SpringAuthorizationEventPublisher {
    pub fn new(event_publisher: Arc<dyn ApplicationEventPublisher>) -> Self {
        Self {
            event_publisher,
            should_publish: Box::new(|result: &dyn AuthorizationResult| !result.is_granted()),
        }
    }

    pub fn set_should_publish(
        &mut self,
        should_publish: impl Fn(&dyn AuthorizationResult) -> bool + Send + Sync + 'static,
    ) {
        self.should_publish = Box::new(should_publish);
    }
}

impl AuthorizationEventPublisher for SpringAuthorizationEventPublisher {
    fn publish_authorization_event(
        &self,
        authentication: Arc<dyn Authentication>,
        object_description: (),
        result: Option<Box<dyn AuthorizationResult>>,
    ) {
        // let result = match result {
        //     Some(r) => r,
        //     None => return,
        // };

        // if !(self.should_publish)(result.as_ref()) {
        //     return;
        // }

        // let event =
        //     AuthorizationDeniedEvent::new(authentication, object_description.to_string(), result);
        // self.event_publisher.publish_event(Box::new(event));
        //
        todo!()
    }
}
