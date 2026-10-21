//! A no-op state listener, meant to be overridden selectively.

use next_web_core::any::AnyValue;
use next_web_core::messaging::Message;

use crate::StateMachine;
use crate::listener::state_machine_listener::StateMachineListener;
use crate::state::StateMachineState;
use crate::state_context::StateContext;
use crate::transition::StateMachineTransition;

/// Adapter for [`StateMachineListener`].
///
/// Every callback does nothing, so an implementation only has to override the
/// callbacks it cares about. This mirrors the Java
/// `StateMachineListenerAdapter`.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct StateMachineListenerAdapter<S, E> {
    _marker: std::marker::PhantomData<(S, E)>,
}

impl<S, E> Default for StateMachineListenerAdapter<S, E> {
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S, E> StateMachineListenerAdapter<S, E> {
    /// Creates a new adapter.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<S, E> StateMachineListener<S, E> for StateMachineListenerAdapter<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn state_changed(&self, _from: &dyn StateMachineState<S, E>, _to: &dyn StateMachineState<S, E>) {}

    fn state_entered(&self, _state: &dyn StateMachineState<S, E>) {}

    fn state_exited(&self, _state: &dyn StateMachineState<S, E>) {}

    fn event_not_accepted(&self, _event: &dyn Message<E>) {}

    fn transition(&self, _transition: &dyn StateMachineTransition<S, E>) {}

    fn transition_started(&self, _transition: &dyn StateMachineTransition<S, E>) {}

    fn transition_ended(&self, _transition: &dyn StateMachineTransition<S, E>) {}

    fn state_machine_started(&self, _state_machine: &dyn StateMachine<S, E>) {}

    fn state_machine_stopped(&self, _state_machine: &dyn StateMachine<S, E>) {}

    fn state_machine_error(&self, _state_machine: &dyn StateMachine<S, E>, _error: &dyn std::error::Error) {
    }

    fn extended_state_changed(&self, _key: &str, _value: &AnyValue) {}

    fn state_context(&self, _state_context: &dyn StateContext<S, E>) {}
}
