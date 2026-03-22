use next_web_core::anys::any_value::AnyValue;
use next_web_core::traits::message::Message;
use std::sync::Arc;

use crate::state_machine::listener::base_composite_listener::BaseCompositeListener;
use crate::state_machine::listener::state_machine_listener::StateMachineListener;
use crate::state_machine::state::StateMachineState;
use crate::state_machine::state_context::StateContext;
use crate::state_machine::transition::StateMachineTransition;
use crate::state_machine::StateMachine;

/// Default state machine listener dispatcher.
///
/// This struct manages a collection of state machine listeners and dispatches
/// events to all registered listeners in reverse order.
pub struct CompositeStateMachineListener<S, E> {
    base: BaseCompositeListener<Arc<dyn StateMachineListener<S, E>>>,
}

impl<S, E> CompositeStateMachineListener<S, E> {
    /// Returns a copy of the current listeners.
    ///
    /// This is useful for iterating over listeners without holding the lock.
    pub fn get_listeners(&self) -> &Vec<Arc<dyn StateMachineListener<S, E>>> {
        &self.base.get_listeners().ordered
    }
}

impl<S, E> Default for CompositeStateMachineListener<S, E> {
    fn default() -> Self {
        Self {
            base: Default::default(),
        }
    }
}

impl<S, E> Clone for CompositeStateMachineListener<S, E>
where
    S: Clone,
    E: Clone,
{
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
        }
    }
}

impl<S, E> StateMachineListener<S, E> for CompositeStateMachineListener<S, E> {
    fn state_changed(&self, from: &dyn StateMachineState<S, E>, to: &dyn StateMachineState<S, E>) {
        for listener in self.get_listeners().iter().rev() {
            listener.state_changed(from, to);
        }
    }

    fn state_entered(&self, state: &dyn StateMachineState<S, E>) {
        for listener in self.get_listeners().iter().rev() {
            listener.state_entered(state);
        }
    }

    fn state_exited(&self, state: &dyn StateMachineState<S, E>) {
        for listener in self.get_listeners().iter().rev() {
            listener.state_exited(state);
        }
    }

    fn event_not_accepted(&self, event: &dyn Message<E>) {
        for listener in self.get_listeners().iter().rev() {
            listener.event_not_accepted(event);
        }
    }

    fn transition(&self, transition: &dyn StateMachineTransition<S, E>) {
        for listener in self.get_listeners().iter().rev() {
            listener.transition(transition);
        }
    }

    fn transition_started(&self, transition: &dyn StateMachineTransition<S, E>) {
        for listener in self.get_listeners().iter().rev() {
            listener.transition_started(transition);
        }
    }

    fn transition_ended(&self, transition: &dyn StateMachineTransition<S, E>) {
        for listener in self.get_listeners().iter().rev() {
            listener.transition_ended(transition);
        }
    }

    fn state_machine_started(&self, state_machine: &dyn StateMachine<S, E>) {
        for listener in self.get_listeners().iter().rev() {
            listener.state_machine_started(state_machine);
        }
    }

    fn state_machine_stopped(&self, state_machine: &dyn StateMachine<S, E>) {
        for listener in self.get_listeners().iter().rev() {
            listener.state_machine_stopped(state_machine);
        }
    }

    fn state_machine_error(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        error: &dyn std::error::Error,
    ) {
        for listener in self.get_listeners().iter().rev() {
            listener.state_machine_error(state_machine, error);
        }
    }

    fn extended_state_changed(&self, key: &str, value: &AnyValue) {
        for listener in self.get_listeners().iter().rev() {
            listener.extended_state_changed(key, value);
        }
    }

    fn state_context(&self, state_context: &dyn StateContext<S, E>) {
        for listener in self.get_listeners().iter().rev() {
            listener.state_context(state_context);
        }
    }
}
