use crate::traits::event::application_event::ApplicationEvent;

#[derive(Clone)]
pub struct ApplicationEventWrapper<T: ApplicationEvent> {
    application_event: T,
    attributes: EventAttributes,
}

impl<T: ApplicationEvent> ApplicationEventWrapper<T> {
    pub fn new(application_event: T) -> Self {
        Self {
            application_event,
            attributes: Default::default(),
        }
    }

    pub fn attributes(&self) -> &EventAttributes {
        &self.attributes
    }
}

impl<T> ApplicationEvent for ApplicationEventWrapper<T>
where
    T: ApplicationEvent,
    T: Clone,
{
    fn timestamp(&self) -> u64 {
        let t1 = self.application_event.timestamp();
        if t1 == 0 {
            self.attributes.timestamp
        } else {
            t1
        }
    }

    fn source(&self) -> &'static str {
        self.application_event.source()
    }
}

#[derive(Debug, Clone)]
pub struct EventAttributes {
    timestamp: u64,
}

impl EventAttributes {
    pub fn new(timestamp: u64) -> Self {
        Self { timestamp }
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }
}

impl Default for EventAttributes {
    fn default() -> Self {
        Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}
