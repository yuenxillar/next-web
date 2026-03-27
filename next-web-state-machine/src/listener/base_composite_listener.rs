use next_web_core::traits::ordered::Ordered;

use crate::listener::ordered_composite::OrderedComposite;

/// Base implementation for all composite listeners.
#[derive(Clone)]
pub struct BaseCompositeListener<T> {
    listeners: OrderedComposite<T>,
}

impl<T> BaseCompositeListener<T> {
    /// Gets the listeners.
    pub fn get_listeners(&self) -> &OrderedComposite<T> {
        &self.listeners
    }

    pub fn get_mut_listeners(&mut self) -> &mut OrderedComposite<T> {
        &mut self.listeners
    }
}

impl<T> BaseCompositeListener<T>
where
    T: Ordered + Eq,
{
    /// Sets the list of listeners. This clears all existing listeners.
    pub fn set_listeners(&mut self, new_listeners: Vec<T>) {
        self.listeners.set_items(new_listeners);
    }

    /// Register a new listener.
    pub fn register(&mut self, listener: T) {
        self.listeners.add(listener);
    }

    /// Unregister a listener.
    pub fn unregister(&mut self, listener: &T) {
        self.listeners.remove(listener);
    }
}

impl<T> Default for BaseCompositeListener<T> {
    fn default() -> Self {
        Self {
            listeners: OrderedComposite::default(),
        }
    }
}
