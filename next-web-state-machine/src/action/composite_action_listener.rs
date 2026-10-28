//! A listener that fans out action execution events to a set of listeners.

use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

use next_web_core::async_trait;

use crate::BoxedStateAction;
use crate::state::action_listener::ActionListener;
use crate::state_machine::StateMachine;
use crate::support::lock_util::{read, write};

/// A composite of [`ActionListener`]s.
///
/// The Java original keeps a `CompositeActionListener` on every state and
/// transition and registers several listeners into it; the Rust port uses this
/// type for the same purpose.
pub struct CompositeActionListener<S, E> {
    listeners: RwLock<Arc<Vec<Arc<dyn ActionListener<S, E>>>>>,
}

impl<S, E> Default for CompositeActionListener<S, E> {
    fn default() -> Self {
        Self {
            listeners: RwLock::new(Arc::new(Vec::new())),
        }
    }
}

impl<S, E> CompositeActionListener<S, E>
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
    pub fn register(&self, listener: Arc<dyn ActionListener<S, E>>) {
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
    pub fn unregister(&self, listener: Arc<dyn ActionListener<S, E>>) {
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
    pub fn snapshot(&self) -> Arc<Vec<Arc<dyn ActionListener<S, E>>>> {
        read(&self.listeners).clone()
    }

    /// Notifies every listener that an action has been executed.
    ///
    /// # Arguments
    /// * `state_machine` - the state machine that executed the action
    /// * `action` - the executed action
    /// * `duration` - the execution duration
    pub async fn notify(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        action: &BoxedStateAction<S, E>,
        duration: Duration,
    ) {
        let listeners = self.snapshot();
        for listener in listeners.iter() {
            listener.on_execute(state_machine, action, duration).await;
        }
    }
}

#[async_trait]
impl<S, E> ActionListener<S, E> for CompositeActionListener<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    async fn on_execute(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        action: &BoxedStateAction<S, E>,
        duration: Duration,
    ) {
        self.notify(state_machine, action, duration).await;
    }
}
