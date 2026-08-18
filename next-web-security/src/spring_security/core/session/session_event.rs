use std::any::{Any, TypeId};

use next_web_context::ApplicationEvent;

use crate::web::session::{HttpSessionDestroyedEvent, HttpSessionIdChangedEvent};

#[derive(Clone)]
pub enum SessionEvent {
    Created(()),
    Destroyed(HttpSessionDestroyedEvent),
    IdChanged(HttpSessionIdChangedEvent),
}

impl ApplicationEvent for SessionEvent {
    fn source(&self) -> &dyn Any {
        match self {
            SessionEvent::Created(_) => self,
            SessionEvent::Destroyed(event) => event.source(),
            SessionEvent::IdChanged(event) => event.source(),
        }
    }

    fn event_type(&self) -> TypeId {
        TypeId::of::<SessionEvent>()
    }

    fn source_type(&self) -> TypeId {
        match self {
            SessionEvent::Created(_) => TypeId::of::<()>(),
            SessionEvent::Destroyed(event) => event.source_type(),
            SessionEvent::IdChanged(event) => event.source_type(),
        }
    }

    fn timestamp(&self) -> u64 {
        match self {
            SessionEvent::Created(_) => 0,
            SessionEvent::Destroyed(event) => event.timestamp(),
            SessionEvent::IdChanged(event) => event.timestamp(),
        }
    }
}
