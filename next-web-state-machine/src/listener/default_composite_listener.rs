use std::sync::Arc;

use crate::state::pseudo_state_listener::PseudoStateListener;

/// Base implementation for all composite listeners.
#[derive(Clone)]
pub struct DefaultCompositeListener<S, E> {
    pub(crate) listeners: Vec<Arc<dyn PseudoStateListener<S, E>>>,
}

impl<S, E> DefaultCompositeListener<S, E> {
    /// Gets the listeners.
    pub fn get_listeners(&self) -> Vec<&dyn PseudoStateListener<S, E>> {
        self.listeners.iter().map(AsRef::as_ref).collect()
    }

    /// Sets the list of listeners. This clears all existing listeners.
    pub fn set_listeners(&mut self, listeners: Vec<Arc<dyn PseudoStateListener<S, E>>>) {
        self.listeners.clear();

        for item in listeners {
            self.register(item);
        }
    }

    /// Register a new listener.
    pub fn register(&mut self, listener: Arc<dyn PseudoStateListener<S, E>>) {
        if !self
            .listeners
            .iter()
            .find(|i| Arc::ptr_eq(i, &listener))
            .is_some()
        {
            self.listeners.push(listener);
            self.listeners.sort_by(|a, b| a.order().cmp(&b.order()));
        }
    }

    /// Unregister a listener.
    pub fn unregister(&mut self, listener: &Arc<dyn PseudoStateListener<S, E>>) {
        self.listeners.retain(|i| !Arc::ptr_eq(i, listener));
    }
}

impl<S, E> Default for DefaultCompositeListener<S, E> {
    fn default() -> Self {
        Self {
            listeners: Default::default(),
        }
    }
}
