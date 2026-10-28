//! The listener of an event bus.

use crate::core::util::object::Object;

/// A component that is told about the events an application publishes.
///
/// An event is an [`Object`], so a bus delivers whatever value the publisher
/// builds: the events of the framework are the values of the event module, and
/// an application can publish its own.
pub trait EventListener: Send + Sync {
    /// Handles an event.
    ///
    /// # Arguments
    ///
    /// * `event` - The event that was published.
    fn on_event(&self, event: &Object);
}
