use next_web_core::any::AnyValue;
use next_web_core::messaging::Message;
use std::sync::{Arc, RwLock};

use crate::StateMachine;
use crate::listener::state_machine_listener::StateMachineListener;
use crate::state::StateMachineState;
use crate::state_context::StateContext;
use crate::support::lock_util::{read, write};
use crate::transition::StateMachineTransition;

/// Default state machine listener dispatcher.
///
/// The listener list lives behind a lock so listeners can be attached to a
/// running machine through `Region::add_state_listener`. Registration replaces
/// the list copy-on-write, which means a notification clones one [`Arc`]
/// instead of every listener.
pub struct CompositeStateMachineListener<S, E> {
    listeners: Arc<RwLock<Arc<Vec<Arc<dyn StateMachineListener<S, E>>>>>>,
}

impl<S, E> CompositeStateMachineListener<S, E> {
    /// Gets a snapshot of the registered listeners.
    ///
    /// The snapshot is an [`Arc`] clone, no listener is cloned.
    pub fn snapshot(&self) -> Arc<Vec<Arc<dyn StateMachineListener<S, E>>>> {
        read(&self.listeners).clone()
    }

    /// Gets the registered listeners.
    pub fn get_listeners(&self) -> Vec<Arc<dyn StateMachineListener<S, E>>> {
        self.snapshot().as_ref().clone()
    }

    /// Registers a listener.
    ///
    /// # Arguments
    /// * `listener` - the listener to register
    pub fn register(&self, listener: Arc<dyn StateMachineListener<S, E>>) {
        let mut guard = write(&self.listeners);

        if guard
            .iter()
            .any(|existing| Arc::ptr_eq(existing, &listener))
        {
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
    pub fn unregister(&self, listener: &dyn StateMachineListener<S, E>) {
        let mut guard = write(&self.listeners);
        let target = listener as *const dyn StateMachineListener<S, E>;
        let next: Vec<_> = guard
            .iter()
            .filter(|existing| !std::ptr::addr_eq(Arc::as_ptr(existing), target))
            .cloned()
            .collect();
        *guard = Arc::new(next);
    }
}

impl<S, E> Default for CompositeStateMachineListener<S, E> {
    fn default() -> Self {
        Self {
            listeners: Arc::new(RwLock::new(Arc::new(Vec::new()))),
        }
    }
}

impl<S, E> Clone for CompositeStateMachineListener<S, E> {
    fn clone(&self) -> Self {
        Self {
            listeners: self.listeners.clone(),
        }
    }
}

impl<S, E> StateMachineListener<S, E> for CompositeStateMachineListener<S, E> {
    fn state_changed(&self, from: &dyn StateMachineState<S, E>, to: &dyn StateMachineState<S, E>) {
        let listeners = self.snapshot();
        for listener in listeners.iter().rev() {
            listener.state_changed(from, to);
        }
    }

    fn state_entered(&self, state: &dyn StateMachineState<S, E>) {
        let listeners = self.snapshot();
        for listener in listeners.iter().rev() {
            listener.state_entered(state);
        }
    }

    fn state_exited(&self, state: &dyn StateMachineState<S, E>) {
        let listeners = self.snapshot();
        for listener in listeners.iter().rev() {
            listener.state_exited(state);
        }
    }

    fn event_not_accepted(&self, event: &dyn Message<E>) {
        let listeners = self.snapshot();
        for listener in listeners.iter().rev() {
            listener.event_not_accepted(event);
        }
    }

    fn transition(&self, transition: &dyn StateMachineTransition<S, E>) {
        let listeners = self.snapshot();
        for listener in listeners.iter().rev() {
            listener.transition(transition);
        }
    }

    fn transition_started(&self, transition: &dyn StateMachineTransition<S, E>) {
        let listeners = self.snapshot();
        for listener in listeners.iter().rev() {
            listener.transition_started(transition);
        }
    }

    fn transition_ended(&self, transition: &dyn StateMachineTransition<S, E>) {
        let listeners = self.snapshot();
        for listener in listeners.iter().rev() {
            listener.transition_ended(transition);
        }
    }

    fn state_machine_started(&self, state_machine: &dyn StateMachine<S, E>) {
        let listeners = self.snapshot();
        for listener in listeners.iter().rev() {
            listener.state_machine_started(state_machine);
        }
    }

    fn state_machine_stopped(&self, state_machine: &dyn StateMachine<S, E>) {
        let listeners = self.snapshot();
        for listener in listeners.iter().rev() {
            listener.state_machine_stopped(state_machine);
        }
    }

    fn state_machine_error(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        error: &dyn std::error::Error,
    ) {
        let listeners = self.snapshot();
        for listener in listeners.iter().rev() {
            listener.state_machine_error(state_machine, error);
        }
    }

    fn extended_state_changed(&self, key: &str, value: &AnyValue) {
        let listeners = self.snapshot();
        for listener in listeners.iter().rev() {
            listener.extended_state_changed(key, value);
        }
    }

    fn state_context(&self, state_context: &dyn StateContext<S, E>) {
        let listeners = self.snapshot();
        for listener in listeners.iter().rev() {
            listener.state_context(state_context);
        }
    }
}
