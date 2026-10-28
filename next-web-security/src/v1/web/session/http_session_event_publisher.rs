//! The publisher of the events of the sessions of the container.

use std::sync::Arc;

use next_web_context::{ApplicationEventPublisher, ApplicationEventPublisherExt};
use next_web_core::http::session::{HttpSession, SessionLifecycleListener};

use crate::{
    core::session::SessionEvent,
    web::session::{HttpSessionCreatedEvent, HttpSessionDestroyedEvent},
};

/// Publishes the events of the sessions of the container on the publisher of the
/// application.
///
/// The publisher of the sessions is a listener of the life of a session: the store
/// of the sessions tells it when a session is created and when it is destroyed, and
/// it publishes the event of that to the listeners of the application. It is what
/// carries the life of a session to the parts of the application that follow it —
/// the registry of the sessions, which is what the control of the concurrent
/// sessions of a user is built on, is one of them.
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::Arc;
///
/// use next_web_security::web::session::HttpSessionEventPublisher;
///
/// manager.add_lifecycle_listener(Arc::new(HttpSessionEventPublisher::new(publisher)));
/// ```
pub struct HttpSessionEventPublisher {
    /// The publisher of the application, which the events are published on.
    publisher: Arc<dyn ApplicationEventPublisher>,
}

impl HttpSessionEventPublisher {
    /// Creates the publisher of the events of the sessions.
    ///
    /// # Arguments
    ///
    /// * `publisher` - The publisher of the application.
    pub fn new(publisher: Arc<dyn ApplicationEventPublisher>) -> Self {
        Self { publisher }
    }

    /// Publishes the event of a session.
    ///
    /// The failure of the listeners of an event is the failure of the publishing
    /// call, and a publisher of the events of the sessions cannot fail the request
    /// that created or destroyed a session, so the failure is logged.
    ///
    /// # Arguments
    ///
    /// * `event` - The event of the session.
    fn publish(&self, event: SessionEvent) {
        if let Err(error) = self.publisher.publish(event) {
            tracing::warn!(
                error = %error,
                "the event of a session was not delivered to its listeners"
            );
        }
    }
}

impl std::fmt::Debug for HttpSessionEventPublisher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpSessionEventPublisher").finish_non_exhaustive()
    }
}

impl SessionLifecycleListener for HttpSessionEventPublisher {
    fn session_created(&self, session: Arc<dyn HttpSession>) {
        self.publish(SessionEvent::Created(HttpSessionCreatedEvent::new(session)));
    }

    fn session_destroyed(&self, session: Arc<dyn HttpSession>) {
        self.publish(SessionEvent::Destroyed(HttpSessionDestroyedEvent::new(
            session,
        )));
    }
}

#[cfg(test)]
mod tests {
    use std::{
        any::Any,
        fmt,
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use next_web_context::{ApplicationEvent, ApplicationEventPublisher, BoxError};
    use next_web_core::{any::AnyValue, http::session::HttpSession};

    use super::HttpSessionEventPublisher;
    use crate::core::session::SessionEvent;

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

    /// The publisher of the tests: it records the events of the sessions.
    #[derive(Default)]
    struct RecordingPublisher {
        /// The number of events that were published.
        published: AtomicUsize,
        /// The kinds of the events that were published, in order.
        kinds: Mutex<Vec<&'static str>>,
    }

    impl RecordingPublisher {
        /// Returns the kinds of the events that were published.
        fn kinds(&self) -> Vec<&'static str> {
            self.kinds
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
        }
    }

    impl ApplicationEventPublisher for RecordingPublisher {
        fn publish_event(&self, event: Arc<dyn ApplicationEvent>) -> Result<(), BoxError> {
            let kind = match (event.as_ref() as &dyn Any).downcast_ref::<SessionEvent>() {
                Some(SessionEvent::Created(_)) => "created",
                Some(SessionEvent::Destroyed(_)) => "destroyed",
                Some(SessionEvent::IdChanged(_)) => "changed",
                None => "other",
            };

            self.published.fetch_add(1, Ordering::SeqCst);
            self.kinds
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(kind);

            Ok(())
        }
    }

    /// Returns a session of the tests.
    fn session(id: &str) -> Arc<dyn HttpSession> {
        Arc::new(TestSession {
            id: id.to_owned(),
        })
    }

    #[test]
    fn the_life_of_a_session_is_published_to_the_application() {
        use next_web_core::http::session::SessionLifecycleListener;

        let publisher = Arc::new(RecordingPublisher::default());
        let events = HttpSessionEventPublisher::new(publisher.clone());

        events.session_created(session("session-1"));
        events.session_destroyed(session("session-1"));

        assert_eq!(publisher.kinds(), vec!["created", "destroyed"]);
        assert_eq!(publisher.published.load(Ordering::SeqCst), 2);
    }
}
