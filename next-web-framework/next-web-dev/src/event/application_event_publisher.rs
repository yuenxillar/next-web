use std::any::TypeId;

use super::application_event::ApplicationEvent;
use async_trait::async_trait;

#[async_trait]
pub trait ApplicationEventPublisher<E>: Send + Sync
where
    E: ApplicationEvent,
{
    fn eid(&self) -> TypeId {
        TypeId::of::<E>()
    }

    fn id(&self) -> String;

    fn channel(&self) -> Option<&tokio::sync::mpsc::Sender<E>>;

    fn set_channel(&mut self, channel: Option<tokio::sync::mpsc::Sender<E>>);

    async fn publish_event(&self, event: E) -> Result<(), Box<dyn std::error::Error>> {
        let _ = if let Some(sender) = self.channel() {
            sender.send(event).await?
        };
        Ok(())
    }
}
