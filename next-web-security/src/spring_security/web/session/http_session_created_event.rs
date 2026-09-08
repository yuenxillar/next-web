use std::{
    any::{Any, TypeId},
    sync::Arc,
};

use next_web_context::{ApplicationEvent, EventAttributes};
use next_web_core::traits::http::HttpSession;

/// Published by the HttpSessionEventPublisher when an HttpSession is created by the container
#[derive(Clone)]
pub struct HttpSessionCreatedEvent {
    base: EventAttributes<Arc<dyn HttpSession>>,
}

impl ApplicationEvent for HttpSessionCreatedEvent {
    fn event_type(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn source_type(&self) -> TypeId {
        self.base.source_type()
    }

    fn source(&self) -> &dyn Any {
        self.base.source()
    }

    fn timestamp(&self) -> u64 {
        self.base.timestamp()
    }
}
