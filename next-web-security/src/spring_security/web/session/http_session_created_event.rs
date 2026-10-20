use std::{any::Any, sync::Arc};

use next_web_context::{ApplicationEvent, EventAttributes};
use next_web_core::http::session::HttpSession;

/// The event of the session that was created.
///
/// The event is a wrapper: the session is its source, so the type the event reports
/// as its source is the type of the session it carries, and a listener that reads
/// the session downcasts the source of the event.
#[derive(Clone)]
pub struct HttpSessionCreatedEvent {
    base: EventAttributes<Arc<dyn HttpSession>>,
}

impl HttpSessionCreatedEvent {
    /// Creates the event of the session that was created.
    ///
    /// # Arguments
    ///
    /// * `session` - The session that was created.
    pub fn new(session: Arc<dyn HttpSession>) -> Self {
        Self {
            base: EventAttributes::new(session),
        }
    }

    /// Returns the session that was created.
    pub fn session(&self) -> &dyn HttpSession {
        self.base.value().as_ref()
    }
}

impl ApplicationEvent for HttpSessionCreatedEvent {
    fn source_type(&self) -> std::any::TypeId {
        self.base.source_type()
    }

    fn source(&self) -> &dyn Any {
        self.base.source()
    }

    fn timestamp(&self) -> u64 {
        self.base.timestamp()
    }
}
