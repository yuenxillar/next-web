use next_web_core::anys::any_value::AnyValue;
use next_web_core::traits::message::Message;
use std::sync::{Arc, RwLock};

use crate::listener::state_machine_listener::StateMachineListener;
use crate::state::StateMachineState;
use crate::state_context::StateContext;
use crate::transition::StateMachineTransition;
use crate::StateMachine;

/// Default state machine listener dispatcher.
///
/// This implementation keeps an internally mutable listener list so listeners
/// can be attached to a running machine through `Region::add_state_listener(&self)`.
pub struct CompositeStateMachineListener<S, E> {
    listeners: Arc<RwLock<Vec<Arc<dyn StateMachineListener<S, E>>>>>,
}

impl<S, E> CompositeStateMachineListener<S, E> {
    fn snapshot(&self) -> Vec<Arc<dyn StateMachineListener<S, E>>> {
        self.listeners
            .read()
            .expect("state machine listener registry poisoned")
            .clone()
    }

    pub fn get_listeners(&self) -> Vec<Arc<dyn StateMachineListener<S, E>>> {
        self.snapshot()
    }

    pub fn register(&self, listener: Arc<dyn StateMachineListener<S, E>>) {
        let mut listeners = self
            .listeners
            .write()
            .expect("state machine listener registry poisoned");

        if listeners
            .iter()
            .any(|existing| Arc::ptr_eq(existing, &listener))
        {
            return;
        }

        listeners.push(listener);
    }

    pub fn unregister(&self, listener: &dyn StateMachineListener<S, E>) {
        let mut listeners = self
            .listeners
            .write()
            .expect("state machine listener registry poisoned");
        let target = listener as *const dyn StateMachineListener<S, E>;

        listeners.retain(|existing| !std::ptr::addr_eq(Arc::as_ptr(existing), target));
    }
}

impl<S, E> Default for CompositeStateMachineListener<S, E> {
    fn default() -> Self {
        Self {
            listeners: Arc::new(RwLock::new(Vec::new())),
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
        for listener in self.snapshot().iter().rev() {
            listener.state_changed(from, to);
        }
    }

    fn state_entered(&self, state: &dyn StateMachineState<S, E>) {
        for listener in self.snapshot().iter().rev() {
            listener.state_entered(state);
        }
    }

    fn state_exited(&self, state: &dyn StateMachineState<S, E>) {
        for listener in self.snapshot().iter().rev() {
            listener.state_exited(state);
        }
    }

    fn event_not_accepted(&self, event: &dyn Message<E>) {
        for listener in self.snapshot().iter().rev() {
            listener.event_not_accepted(event);
        }
    }

    fn transition(&self, transition: &dyn StateMachineTransition<S, E>) {
        for listener in self.snapshot().iter().rev() {
            listener.transition(transition);
        }
    }

    fn transition_started(&self, transition: &dyn StateMachineTransition<S, E>) {
        for listener in self.snapshot().iter().rev() {
            listener.transition_started(transition);
        }
    }

    fn transition_ended(&self, transition: &dyn StateMachineTransition<S, E>) {
        for listener in self.snapshot().iter().rev() {
            listener.transition_ended(transition);
        }
    }

    fn state_machine_started(&self, state_machine: &dyn StateMachine<S, E>) {
        for listener in self.snapshot().iter().rev() {
            listener.state_machine_started(state_machine);
        }
    }

    fn state_machine_stopped(&self, state_machine: &dyn StateMachine<S, E>) {
        for listener in self.snapshot().iter().rev() {
            listener.state_machine_stopped(state_machine);
        }
    }

    fn state_machine_error(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        error: &dyn std::error::Error,
    ) {
        for listener in self.snapshot().iter().rev() {
            listener.state_machine_error(state_machine, error);
        }
    }

    fn extended_state_changed(&self, key: &str, value: &AnyValue) {
        for listener in self.snapshot().iter().rev() {
            listener.extended_state_changed(key, value);
        }
    }

    fn state_context(&self, state_context: &dyn StateContext<S, E>) {
        for listener in self.snapshot().iter().rev() {
            listener.state_context(state_context);
        }
    }
}
