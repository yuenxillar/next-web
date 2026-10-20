use std::sync::Arc;

use super::application_event::ApplicationEvent;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Application event publisher
pub trait ApplicationEventPublisher
where
    Self: Send + Sync,
{
    /// Publish event
    fn publish_event(&self, event: Arc<dyn ApplicationEvent>) -> Result<(), BoxError>;
}
