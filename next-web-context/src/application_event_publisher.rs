use std::sync::Arc;

use super::application_event::ApplicationEvent;

/// The error the publishing of an event reports.
///
/// The failure of the listeners of an event is the failure of the publishing call,
/// and a publisher that reports it answers with it.
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Publishes an event of the application.
///
/// The event is published once and shared with every listener, which is what makes
/// an event that is not cloneable publishable at all. A publisher that has an event
/// of a type of its own uses [`publish`](ApplicationEventPublisherExt::publish),
/// which shares the value for it; a publisher that already holds the shared event
/// (a publisher that publishes what it received) uses
/// [`publish_shared`](ApplicationEventPublisherExt::publish_shared).
pub trait ApplicationEventPublisher
where
    Self: Send + Sync,
{
    /// Publishes the event, which the listeners of the application receive.
    ///
    /// # Arguments
    ///
    /// * `event` - The event, which is shared with every listener.
    fn publish_event(&self, event: Arc<dyn ApplicationEvent>) -> Result<(), BoxError>;
}
