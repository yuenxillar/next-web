use std::any::TypeId;

use super::application_event::ApplicationEvent;
use async_trait::async_trait;

#[async_trait]
pub trait ApplicationListener<E>: Send + Sync + 'static
where
    E: ApplicationEvent,
{
    fn eid(&self) -> TypeId {
        TypeId::of::<E>()
    }

    fn id(&self) -> String;

    async fn on_application_event(&mut self, event: E);
}
