use crate::event::default_application_event_multicaster::DefaultApplicationEventMulticaster;
use next_web_core::{
    async_trait,
    error::BoxError,
    traits::event::{
        application_event::ApplicationEvent,
        application_event_multicaster::ApplicationEventMulticaster,
        application_event_publisher::ApplicationEventPublisher,
    },
};

#[derive(Clone)]
pub struct DefaultApplicationEventPublisher<T = DefaultApplicationEventMulticaster> {
    multicaster: T,
}

impl<T> DefaultApplicationEventPublisher<T>
where
    T: ApplicationEventMulticaster,
{
    pub fn new(multicaster: T) -> Self {
        Self { multicaster }
    }
}

#[async_trait]
impl<T> ApplicationEventPublisher for DefaultApplicationEventPublisher<T>
where
    T: ApplicationEventMulticaster,
{
    /// 发布事件
    ///
    /// Publish event
    async fn publish_event<E>(&self, event: E) -> Result<(), BoxError>
    where
        E: ApplicationEvent,
    {
        self.multicaster.multicast_event(Box::new(event)).await;

        Ok(())
    }
}

impl<T> Default for DefaultApplicationEventPublisher<T>
where
    T: Default,
{
    fn default() -> Self {
        Self {
            multicaster: Default::default(),
        }
    }
}
