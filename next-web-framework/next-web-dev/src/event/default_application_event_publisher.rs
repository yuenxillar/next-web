use flume::Sender;
use std::borrow::Cow;


use super::{
    application_event::ApplicationEvent, application_event_publisher::ApplicationEventPublisher,
};

#[derive(Clone)]
pub struct DefaultApplicationEventPublisher {
    channel: Option<Sender<(Cow<'static, str>, Box<dyn ApplicationEvent>)>>,
}

impl DefaultApplicationEventPublisher {
    pub fn new() -> Self {
        Self { channel: None }
    }

    pub fn set_channel(&mut self, channel: Option<Sender<(Cow<'static, str>, Box<dyn ApplicationEvent>)>>) {
        self.channel = channel;
    }
}

impl ApplicationEventPublisher for DefaultApplicationEventPublisher {
    /// 发布事件
    /// Publish event
    fn publish_event(
        &self,
        event: Box<dyn ApplicationEvent>
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(sender) = &self.channel {
            sender.send((self.id(), event))?
        };
        Ok(())
    }
}
