//! A listener that fans out pseudo state notifications.

use std::sync::Arc;
use std::sync::RwLock;

use next_web_core::Ordered;

use crate::state::pseudo_state_context::PseudoStateContext;
use crate::state::pseudo_state_listener::PseudoStateListener;
use crate::support::lock_util::{read, write};

/// A composite of [`PseudoStateListener`]s.
///
/// Listeners are notified in reverse registration order, which is what the Java
/// `CompositePseudoStateListener` does.
pub struct CompositePseudoStateListener<S, E> {
    listeners: RwLock<Arc<Vec<Arc<dyn PseudoStateListener<S, E>>>>>,
}

impl<S, E> Default for CompositePseudoStateListener<S, E> {
    fn default() -> Self {
        Self {
            listeners: RwLock::new(Arc::new(Vec::new())),
        }
    }
}

impl<S, E> CompositePseudoStateListener<S, E> {
    /// Creates an empty composite listener.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces all listeners.
    ///
    /// # Arguments
    /// * `listeners` - the new listeners
    pub fn set_listeners(&self, listeners: Vec<Arc<dyn PseudoStateListener<S, E>>>) {
        *write(&self.listeners) = Arc::new(listeners);
    }

    /// Registers a listener.
    ///
    /// # Arguments
    /// * `listener` - the listener to register
    pub fn register(&self, listener: Arc<dyn PseudoStateListener<S, E>>) {
        let mut guard = write(&self.listeners);
        let mut next = Vec::with_capacity(guard.len() + 1);
        next.extend(guard.iter().cloned());
        next.push(listener);
        *guard = Arc::new(next);
    }

    /// Gets a snapshot of the registered listeners.
    pub fn snapshot(&self) -> Arc<Vec<Arc<dyn PseudoStateListener<S, E>>>> {
        read(&self.listeners).clone()
    }
}

impl<S, E> PseudoStateListener<S, E> for CompositePseudoStateListener<S, E> {
    fn on_context(&self, context: &dyn PseudoStateContext<S, E>) {
        let listeners = self.snapshot();
        for listener in listeners.iter().rev() {
            listener.on_context(context);
        }
    }
}

impl<S, E> Ordered for CompositePseudoStateListener<S, E> {
    fn order(&self) -> i32 {
        i32::MAX
    }
}
