use std::{
    any::{Any, TypeId},
    sync::Arc,
};

use next_web_context::{ApplicationEvent, EventAttributes};
use next_web_core::traits::http::HttpSession;

/// Published by the HttpSessionEventPublisher when an HttpSession ID is changed.
#[derive(Clone)]
pub struct HttpSessionIdChangedEvent {
    old_session_id: String,
    new_session_id: String,

    base: EventAttributes<Arc<dyn HttpSession>>,
}

impl HttpSessionIdChangedEvent {
    pub fn new(session: Arc<dyn HttpSession>, old_session_id: impl Into<String>) -> Self {
        Self {
            old_session_id: old_session_id.into(),
            new_session_id: session.id().to_string(),

            base: EventAttributes::new(session),
        }
    }

    pub fn old_session_id(&self) -> &str {
        &self.old_session_id
    }

    pub fn new_session_id(&self) -> &str {
        &self.new_session_id
    }
}

impl ApplicationEvent for HttpSessionIdChangedEvent {
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
