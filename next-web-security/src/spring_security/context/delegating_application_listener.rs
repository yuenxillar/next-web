//! The listener that hands the events of the application to a set of listeners.

use std::sync::Arc;

use next_web_context::{ApplicationEvent, ApplicationListener};
use next_web_core::BoxFuture;

/// Hands every event it receives to each of the listeners it was given.
///
/// The configuration of the application builds listeners while it runs (the
/// registry of the sessions, for example), and it cannot register them on the
/// multicaster of the context, which is built before the configuration runs. It
/// adds them here instead, and the context registers this listener once.
///
/// A listener that is added here is called for every event of the application, so
/// a listener that is only interested in one type of event ignores the others by
/// downcasting them. A listener that is registered on the multicaster itself is
/// filtered by the type of the event before it is called, which is what
/// [`ApplicationListenerRegistration`](next_web_context::event::ApplicationListenerRegistration)
/// is for.
#[derive(Clone, Default)]
pub struct DelegatingApplicationListener {
    listeners: Vec<Arc<dyn ApplicationListener<Arc<dyn ApplicationEvent>>>>,
}

impl DelegatingApplicationListener {
    /// Adds a listener that the events are handed to.
    ///
    /// Adding the listener that is already added leaves the listeners as they are.
    ///
    /// # Arguments
    ///
    /// * `listener` - The listener that receives the events of the application.
    pub fn add_listener(
        &mut self,
        listener: Arc<dyn ApplicationListener<Arc<dyn ApplicationEvent>>>,
    ) {
        if self
            .listeners
            .iter()
            .any(|added| Arc::ptr_eq(added, &listener))
        {
            return;
        }

        self.listeners.push(listener);
    }

    /// Returns the listeners the events are handed to.
    pub fn listeners(&self) -> &[Arc<dyn ApplicationListener<Arc<dyn ApplicationEvent>>>] {
        &self.listeners
    }
}

impl std::fmt::Debug for DelegatingApplicationListener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DelegatingApplicationListener")
            .field("listeners", &self.listeners.len())
            .finish()
    }
}

impl ApplicationListener<Arc<dyn ApplicationEvent>> for DelegatingApplicationListener {
    fn on_application_event<'a>(&'a self, event: Arc<dyn ApplicationEvent>) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            // Every listener receives the event: the listeners of this module are
            // the ones that know which events they handle, and they read the event
            // they were given.
            for listener in self.listeners.iter() {
                listener.on_application_event(event.clone()).await;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use next_web_context::{ApplicationEvent, ApplicationListener};
    use next_web_core::BoxFuture;

    use super::DelegatingApplicationListener;

    /// An event of the test.
    #[derive(Clone)]
    struct Ping;

    impl ApplicationEvent for Ping {
        fn source(&self) -> &dyn std::any::Any {
            self
        }
    }

    /// A listener that counts the events it receives.
    #[derive(Default)]
    struct Counter {
        received: AtomicUsize,
    }

    impl ApplicationListener<Arc<dyn ApplicationEvent>> for Counter {
        fn on_application_event<'a>(
            &'a self,
            _event: Arc<dyn ApplicationEvent>,
        ) -> BoxFuture<'a, ()> {
            Box::pin(async move {
                self.received.fetch_add(1, Ordering::SeqCst);
            })
        }
    }

    #[tokio::test]
    async fn every_event_reaches_every_listener_once() {
        let first = Arc::new(Counter::default());
        let second = Arc::new(Counter::default());
        let first_listener: Arc<dyn ApplicationListener<Arc<dyn ApplicationEvent>>> = first.clone();
        let second_listener: Arc<dyn ApplicationListener<Arc<dyn ApplicationEvent>>> =
            second.clone();
        let mut delegating = DelegatingApplicationListener::default();

        delegating.add_listener(first_listener.clone());
        delegating.add_listener(second_listener);
        // Adding a listener twice does not hand it the event twice.
        delegating.add_listener(first_listener);

        delegating.on_application_event(Arc::new(Ping)).await;

        assert_eq!(first.received.load(Ordering::SeqCst), 1);
        assert_eq!(second.received.load(Ordering::SeqCst), 1);
        assert_eq!(delegating.listeners().len(), 2);
    }

    #[tokio::test]
    async fn a_listener_that_was_not_added_receives_nothing() {
        let delegating = DelegatingApplicationListener::default();

        delegating.on_application_event(Arc::new(Ping)).await;

        assert!(delegating.listeners().is_empty());
    }
}
