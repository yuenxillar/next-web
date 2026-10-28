use std::sync::Arc;

use next_web_core::BoxAny;

use crate::{authorization::authorization_result::AuthorizationResult, core::Authentication};

/// A contract for publishing authorization events
pub trait AuthorizationEventPublisher
where
    Self: Send + Sync,
{
    /// Publish the given details in the form of an event, typically AuthorizationGrantedEvent or AuthorizationDeniedEvent.
    /// Note that success events can be very noisy if enabled by default.
    /// Because of this implementations may choose to drop success events by default.
    fn publish_authorization_event(
        &self,
        authentication: Arc<dyn Authentication>,
        value: BoxAny,
        result: Option<Arc<dyn AuthorizationResult>>,
    );
}
