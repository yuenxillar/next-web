use flume::Sender;
use next_web_core::error::BoxError;

use super::{
    application_event::ApplicationEvent, application_event_publisher::ApplicationEventPublisher,
};

#[derive(Clone)]
pub struct DefaultApplicationEventPublisher {
    channel: Option<Sender<(String, Box<dyn ApplicationEvent>)>>,
}

impl DefaultApplicationEventPublisher {
    pub fn new() -> Self {
        Self { channel: None }
    }

    pub(crate) fn set_channel(
        &mut self,
        channel: Option<Sender<(String, Box<dyn ApplicationEvent>)>>,
    ) {
        self.channel = channel;
    }
}

impl ApplicationEventPublisher for DefaultApplicationEventPublisher {
    /// 发布事件
    /// Publish event
    fn publish_event(
        &self,
        id: impl Into<String>,
        event: Box<dyn ApplicationEvent>,
    ) -> Result<(), BoxError> {
        if let Some(sender) = &self.channel {
            sender.send((id.into(), event))?
        };
        Ok(())
    }
}
