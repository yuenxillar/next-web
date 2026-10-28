use std::sync::Arc;

use crate::core::{
    event::{event_bus::EventBus, event_listener::EventListener},
    util::{destroyable::Destroyable, object::Object},
};

/// The event bus of an application that delivers events in process.
///
/// The listeners are kept in the order they were registered and every published
/// event is handed to each of them.
#[derive(Clone, Default)]
pub struct DefaultEventBus {
    listeners: Vec<Arc<dyn EventListener>>,
}

impl DefaultEventBus {
    /// Creates a bus without a listener.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the listeners of the bus, in the order they are told about an
    /// event.
    pub fn listeners(&self) -> &[Arc<dyn EventListener>] {
        &self.listeners
    }

    /// Returns whether the bus holds no listener at all.
    pub fn is_empty(&self) -> bool {
        self.listeners.is_empty()
    }
}

impl EventBus for DefaultEventBus {
    fn publish(&self, event: Object) {
        for listener in &self.listeners {
            listener.on_event(&event);
        }
    }

    fn register(&mut self, listener: Arc<dyn EventListener>) {
        self.listeners.push(listener);
    }

    fn unregister(&mut self, listener: &Arc<dyn EventListener>) {
        self.listeners
            .retain(|registered| !Arc::ptr_eq(registered, listener));
    }
}

impl Destroyable for DefaultEventBus {
    fn destroy(self) {
        // The listeners belong to the application, the bus only drops its hold on
        // them; dropping the bus is what makes it forget them.
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::DefaultEventBus;
    use crate::core::{
        event::{event_bus::EventBus, event_listener::EventListener},
        util::object::Object,
    };

    /// A listener that records the events it was told about.
    #[derive(Default)]
    struct Recorder {
        events: Mutex<Vec<String>>,
    }

    impl EventListener for Recorder {
        fn on_event(&self, event: &Object) {
            self.events
                .lock()
                .expect("the recorder is not poisoned")
                .push(event.to_string());
        }
    }

    impl Recorder {
        fn events(&self) -> Vec<String> {
            self.events
                .lock()
                .expect("the recorder is not poisoned")
                .clone()
        }
    }

    #[test]
    fn every_listener_is_told_about_a_published_event() {
        let mut bus = DefaultEventBus::new();
        let first = Arc::new(Recorder::default());
        let second = Arc::new(Recorder::default());
        bus.register(first.clone());
        bus.register(second.clone());

        bus.publish(Object::Str("session started".to_string()));

        assert_eq!(first.events(), vec!["session started".to_string()]);
        assert_eq!(second.events(), vec!["session started".to_string()]);
        assert_eq!(bus.listeners().len(), 2);
    }

    #[test]
    fn an_unregistered_listener_is_no_longer_told_about_events() {
        let mut bus = DefaultEventBus::new();
        let recorder = Arc::new(Recorder::default());
        let listener: Arc<dyn EventListener> = recorder.clone();
        bus.register(listener.clone());
        bus.publish(Object::Int(1));

        bus.unregister(&listener);
        bus.publish(Object::Int(2));

        assert_eq!(recorder.events(), vec!["1".to_string()]);
        assert!(bus.is_empty());
    }

    #[test]
    fn a_bus_without_a_listener_publishes_into_nothing() {
        let bus = DefaultEventBus::default();

        bus.publish(Object::Bool(true));

        assert!(bus.is_empty());
    }
}
