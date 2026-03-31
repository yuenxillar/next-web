use std::sync::Arc;

use next_web_core::traits::message::Message;

use crate::{
    StateMachine, access::reactive_state_machine_access::ReactiveStateMachineAccess,
    monitor::state_machine_monitor::StateMachineMonitor,
    support::state_machine_interceptor::StateMachineInterceptor,
};

/// Functional interface exposing `StateMachine` internals.
pub trait StateMachineAccess<S, E>: ReactiveStateMachineAccess<S, E> {
    /// Sets the relay state machine.
    fn set_relay(&self, state_machine: Arc<dyn StateMachine<S, E>>);

    /// Adds the state machine interceptor.
    fn add_state_machine_interceptor(&self, interceptor: Arc<dyn StateMachineInterceptor<S, E>>);

    /// Adds the state machine monitor.
    fn add_state_machine_monitor(&self, monitor: Arc<dyn StateMachineMonitor<S, E>>);

    /// Sets if initial state is enabled when a state machine is
    /// using sub states.
    fn set_initial_enabled(&self, enabled: bool);

    /// Set initial forwarded event which is used for passing in
    /// event and its headers for actions executed when sub state
    /// is entered via initial transition.
    fn set_forwarded_initial_event(&self, message: Box<dyn Message<E>>);

    /// Sets the parent machine.
    fn set_parent_machine(&self, state_machine: Arc<dyn StateMachine<S, E>>);
}
