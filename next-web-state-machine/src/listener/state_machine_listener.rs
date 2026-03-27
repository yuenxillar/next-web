use next_web_core::{anys::any_value::AnyValue, traits::message::Message};

use crate::{
    state::StateMachineState, state_context::StateContext, transition::StateMachineTransition,
    StateMachine,
};

/// `StateMachineListener` for various state machine events.
///
/// This trait defines callbacks for monitoring and responding to events
/// that occur during the lifecycle of a state machine. Implementations
/// can use these callbacks for logging, metrics, auditing, or triggering
/// side effects in response to state machine activities.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
pub trait StateMachineListener<S, E>
where
    Self: Send + Sync,
{
    /// Notified when state is changed.
    ///
    /// # Arguments
    /// * `from` - the source state
    /// * `to` - the target state
    fn state_changed(&self, from: &dyn StateMachineState<S, E>, to: &dyn StateMachineState<S, E>);

    /// Notified when state is entered.
    ///
    /// # Arguments
    /// * `state` - the state
    fn state_entered(&self, state: &dyn StateMachineState<S, E>);

    /// Notified when state is exited.
    ///
    /// # Arguments
    /// * `state` - the state
    fn state_exited(&self, state: &dyn StateMachineState<S, E>);

    /// Notified when event was not accepted.
    ///
    /// # Arguments
    /// * `event` - the event that was not accepted
    fn event_not_accepted(&self, event: &dyn Message<E>);

    /// Notified when transition happened.
    ///
    /// # Arguments
    /// * `transition` - the transition
    fn transition(&self, transition: &dyn StateMachineTransition<S, E>);

    /// Notified when transition started.
    ///
    /// # Arguments
    /// * `transition` - the transition
    fn transition_started(&self, transition: &dyn StateMachineTransition<S, E>);

    /// Notified when transition ended.
    ///
    /// # Arguments
    /// * `transition` - the transition
    fn transition_ended(&self, transition: &dyn StateMachineTransition<S, E>);

    /// Notified when state machine starts.
    ///
    /// # Arguments
    /// * `state_machine` - the state machine
    fn state_machine_started(&self, state_machine: &dyn StateMachine<S, E>);

    /// Notified when state machine stops.
    ///
    /// # Arguments
    /// * `state_machine` - the state machine
    fn state_machine_stopped(&self, state_machine: &dyn StateMachine<S, E>);

    /// Notified when state machine enters error it can't recover from.
    ///
    /// # Arguments
    /// * `state_machine` - the state machine
    /// * `error` - the exception that caused the error
    fn state_machine_error(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        error: &dyn std::error::Error,
    );

    /// Notified when extended state variable is either added, modified or removed.
    ///
    /// # Arguments
    /// * `key` - the variable key
    /// * `value` - the variable value (None if removed)
    fn extended_state_changed(&self, key: &str, value: &AnyValue);

    /// Notified on various `Stage`s about a `StateContext`.
    ///
    /// # Arguments
    /// * `state_context` - the state context
    fn state_context(&self, state_context: &dyn StateContext<S, E>);
}
