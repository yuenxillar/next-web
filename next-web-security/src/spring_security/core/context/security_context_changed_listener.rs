use crate::core::context::security_context_changed_event::SecurityContextChangedEvent;

/// A listener for SecurityContextChangedEvents.
pub trait SecurityContextChangedListener: Send + Sync {
    fn security_context_changed(&self, event: SecurityContextChangedEvent);
}
