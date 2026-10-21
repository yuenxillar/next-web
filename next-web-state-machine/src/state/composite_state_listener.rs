//! A listener that fans out state events to a registered set of listeners.

use std::sync::Arc;

use next_web_core::async_trait;

use crate::state::state_listener::StateListener;
use crate::state_context::StateContext;
use crate::support::lock_util::{read, write};
use std::sync::RwLock;

/// A composite of [`StateListener`]s.
///
/// A notification clones one [`Arc`] to the listener slice instead of cloning
/// every listener, so dispatching to a registered set costs a single atomic
/// increment. Registration copies the slice on write; that is cheap because it
/// only happens while a machine is built.
pub struct CompositeStateListener<S, E> {
    listeners: RwLock<Arc<Vec<Arc<dyn StateListener<S, E>>>>>,
}

impl<S, E> Default for CompositeStateListener<S, E> {
    fn default() -> Self {
        Self {
            listeners: RwLock::new(Arc::new(Vec::new())),
        }
    }
}

impl<S, E> CompositeStateListener<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    /// Creates an empty composite listener.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a listener.
    ///
    /// # Arguments
    /// * `listener` - the listener to register
    pub fn register(&self, listener: Arc<dyn StateListener<S, E>>) {
        let mut guard = write(&self.listeners);
        if guard.iter().any(|item| Arc::ptr_eq(item, &listener)) {
            return;
        }
        let mut next = Vec::with_capacity(guard.len() + 1);
        next.extend(guard.iter().cloned());
        next.push(listener);
        *guard = Arc::new(next);
    }

    /// Unregisters a listener.
    ///
    /// # Arguments
    /// * `listener` - the listener to unregister
    pub fn unregister(&self, listener: Arc<dyn StateListener<S, E>>) {
        let mut guard = write(&self.listeners);
        if let Some(position) = guard.iter().position(|item| Arc::ptr_eq(item, &listener)) {
            let mut next = Vec::with_capacity(guard.len().saturating_sub(1));
            next.extend(
                guard
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != position)
                    .map(|(_, item)| item.clone()),
            );
            *guard = Arc::new(next);
        }
    }

    /// Gets a snapshot of the registered listeners.
    ///
    /// The snapshot is an [`Arc`] clone, no listener is cloned.
    pub fn snapshot(&self) -> Arc<Vec<Arc<dyn StateListener<S, E>>>> {
        read(&self.listeners).clone()
    }
}

#[async_trait]
impl<S, E> StateListener<S, E> for CompositeStateListener<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    async fn on_entry(&self, context: &dyn StateContext<S, E>) {
        let listeners = self.snapshot();
        for listener in listeners.iter() {
            listener.on_entry(context).await;
        }
    }

    async fn on_exit(&self, context: &dyn StateContext<S, E>) {
        let listeners = self.snapshot();
        for listener in listeners.iter() {
            listener.on_exit(context).await;
        }
    }

    async fn on_complete(&self, context: &dyn StateContext<S, E>) {
        let listeners = self.snapshot();
        for listener in listeners.iter() {
            listener.on_complete(context).await;
        }
    }

    async fn do_on_complete(&self, context: &dyn StateContext<S, E>) {
        let listeners = self.snapshot();
        for listener in listeners.iter() {
            listener.do_on_complete(context).await;
        }
    }
}
