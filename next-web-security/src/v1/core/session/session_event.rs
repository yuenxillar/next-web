use std::any::{Any, TypeId};

use next_web_context::ApplicationEvent;

use crate::web::session::{
    HttpSessionCreatedEvent, HttpSessionDestroyedEvent, HttpSessionIdChangedEvent,
};

/// The event of a session that was created, destroyed or whose identifier changed.
///
/// The event is one type, so a listener of the events of the sessions is registered
/// once and reads the kind of the event from the variant it holds. The source of
/// every variant is the session whose event it is, so the type the event reports as
/// its source is the type of that session, and the time it reports is the time of
/// the event it carries.
#[derive(Clone)]
pub enum SessionEvent {
    /// A session was created.
    Created(HttpSessionCreatedEvent),
    /// A session was destroyed.
    Destroyed(HttpSessionDestroyedEvent),
    /// The identifier of a session changed.
    IdChanged(HttpSessionIdChangedEvent),
}

impl ApplicationEvent for SessionEvent {
    fn source(&self) -> &dyn Any {
        match self {
            SessionEvent::Created(event) => event.source(),
            SessionEvent::Destroyed(event) => event.source(),
            SessionEvent::IdChanged(event) => event.source(),
        }
    }

    fn source_type(&self) -> TypeId {
        match self {
            SessionEvent::Created(event) => event.source_type(),
            SessionEvent::Destroyed(event) => event.source_type(),
            SessionEvent::IdChanged(event) => event.source_type(),
        }
    }

    fn timestamp(&self) -> u64 {
        match self {
            SessionEvent::Created(event) => event.timestamp(),
            SessionEvent::Destroyed(event) => event.timestamp(),
            SessionEvent::IdChanged(event) => event.timestamp(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{any::TypeId, fmt, sync::Arc};

    use next_web_context::ApplicationEvent;
    use next_web_core::{any::AnyValue, http::session::HttpSession};

    use super::SessionEvent;
    use crate::web::session::{
        HttpSessionCreatedEvent, HttpSessionDestroyedEvent, HttpSessionIdChangedEvent,
    };

    /// A session of the tests.
    struct TestSession {
        /// The identifier of the session.
        id: String,
    }

    impl fmt::Debug for TestSession {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "TestSession({})", self.id)
        }
    }

    impl HttpSession for TestSession {
        fn creation_time(&self) -> u64 {
            0
        }

        fn id(&self) -> &str {
            &self.id
        }

        fn last_accessed_time(&self) -> u64 {
            0
        }

        fn set_max_inactive_interval(&mut self, _interval: u64) {}

        fn max_inactive_interval(&self) -> u64 {
            0
        }

        fn attribute(&self, _name: &str) -> Option<&AnyValue> {
            None
        }

        fn attribute_names(&self) -> Vec<&str> {
            Vec::new()
        }

        fn set_attribute(&mut self, _name: &str, _value: AnyValue) {}

        fn remove_attribute(&mut self, _name: &str) {}

        fn invalidate(&mut self) {}

        fn is_new(&self) -> bool {
            false
        }
    }

    /// Returns a session of the tests.
    fn session() -> Arc<dyn HttpSession> {
        Arc::new(TestSession {
            id: String::from("session-1"),
        })
    }

    /// Returns the type every event of the sessions reports as its source.
    fn session_type() -> TypeId {
        TypeId::of::<Arc<dyn HttpSession>>()
    }

    /// Returns the events of the sessions of the tests.
    fn events() -> (SessionEvent, SessionEvent, SessionEvent) {
        (
            SessionEvent::Created(HttpSessionCreatedEvent::new(session())),
            SessionEvent::Destroyed(HttpSessionDestroyedEvent::new(session())),
            SessionEvent::IdChanged(HttpSessionIdChangedEvent::new(session(), "session-0")),
        )
    }

    #[test]
    fn every_variant_reports_the_session_as_its_source() {
        for event in [events().0, events().1, events().2] {
            assert_eq!(
                event.source_type(),
                session_type(),
                "the source of the event is the session it carries"
            );
            assert_eq!(
                event.event_type(),
                TypeId::of::<SessionEvent>(),
                "every variant is the event of the sessions"
            );
        }
    }

    #[test]
    fn every_variant_reports_the_time_of_the_event_it_carries() {
        let created = HttpSessionCreatedEvent::new(session());
        let destroyed = HttpSessionDestroyedEvent::new(session());
        let id_changed = HttpSessionIdChangedEvent::new(session(), "session-0");

        assert_eq!(
            SessionEvent::Created(created.clone()).timestamp(),
            created.timestamp(),
            "the time of the variant is the time of the event it carries"
        );
        assert_eq!(
            SessionEvent::Destroyed(destroyed.clone()).timestamp(),
            destroyed.timestamp()
        );
        assert_eq!(
            SessionEvent::IdChanged(id_changed.clone()).timestamp(),
            id_changed.timestamp()
        );
    }

    #[test]
    fn the_event_of_a_created_session_carries_the_session() {
        let event = HttpSessionCreatedEvent::new(session());

        assert_eq!(event.session().id(), "session-1");
        assert_eq!(event.source_type(), session_type());
        assert!(
            event.timestamp() > 1_600_000_000,
            "the event is stamped with the time it happened"
        );
    }
}
