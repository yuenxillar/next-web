use std::sync::Arc;

use crate::core::{event::event_listener::EventListener, util::object::Object};

/// The bus an application publishes its events on.
///
/// A listener is registered once and is told about every event the bus is given
/// until it is unregistered again, which is how the framework reports a session
/// start, a login, or a cache change to the parts of the application that care.
pub trait EventBus: Send + Sync {
    /// Publishes an event to every registered listener.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to publish.
    fn publish(&self, event: Object);

    /// Registers a listener.
    ///
    /// # Arguments
    ///
    /// * `listener` - The listener to tell about the events published from now on.
    fn register(&mut self, listener: Arc<dyn EventListener>);

    /// Unregisters a listener.
    ///
    /// # Arguments
    ///
    /// * `listener` - The listener that is no longer told about the events.
    fn unregister(&mut self, listener: &Arc<dyn EventListener>);
}