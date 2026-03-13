use crate::traits::{event::application_event::ApplicationEvent, id::Id};
use async_trait::async_trait;

#[async_trait]
pub trait ApplicationListener<E>
where
    Self: Send + Sync,
    Self: 'static,
    Self: Id,
    E: ApplicationEvent,
{
    async fn on_application_event(&self, event: &E);
}
