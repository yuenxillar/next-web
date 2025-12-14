use flume::Sender;
use next_web_core::{
    error::BoxError,
    traits::event::{
        application_event::ApplicationEvent, application_event_publisher::ApplicationEventPublisher,
    },
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
        id: impl ToString,
        event: impl ApplicationEvent,
    ) -> Result<(), BoxError> {
        let event = Box::new(event);
        match self.channel.as_ref() {
            Some(channel) => channel.send((id.to_string(), event)).map_err(Into::into),
            None => Ok(()),
        }
    }
}
