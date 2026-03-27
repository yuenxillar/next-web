use next_web_core::anys::any_value::AnyValue;
use next_web_core::async_trait;
use next_web_core::traits::message::Message;
use std::any::Any;

use crate::state::StateMachineState;
use crate::transition::StateMachineTransition;
use crate::StateMachine;

/// Interface for publishing state machine based application events.
///
/// This trait defines methods for publishing various state machine lifecycle events
/// such as state changes, transitions, and machine start/stop events.
#[async_trait]
pub trait StateMachineEventPublisher<S, E>
where
    Self: Send + Sync,
{
    /// Publishes a state changed event.
    ///
    /// # Arguments
    /// * `source` - The component that generated this event
    /// * `source_state` - The source state before the change
    /// * `target_state` - The target state after the change
    async fn publish_state_changed(
        &self,
        source: &dyn Any,
        source_state: &dyn StateMachineState<S, E>,
        target_state: &dyn StateMachineState<S, E>,
    );

    /// Publishes a state entered event.
    ///
    /// # Arguments
    /// * `source` - The component that generated this event
    /// * `state` - The state that was entered
    async fn publish_state_entered(&self, source: &dyn Any, state: &dyn StateMachineState<S, E>);

    /// Publishes a state exited event.
    ///
    /// # Arguments
    /// * `source` - The component that generated this event
    /// * `state` - The state that was exited
    async fn publish_state_exited(&self, source: &dyn Any, state: &dyn StateMachineState<S, E>);

    /// Publishes an event not accepted event.
    ///
    /// # Arguments
    /// * `source` - The component that generated this event
    /// * `event` - The event that was not accepted
    async fn publish_event_not_accepted(&self, source: &dyn Any, event: &dyn Message<E>);

    /// Publishes a transition start event.
    ///
    /// # Arguments
    /// * `source` - The component that generated this event
    /// * `transition` - The transition that is starting
    async fn publish_transition_start(
        &self,
        source: &dyn Any,
        transition: &dyn StateMachineTransition<S, E>,
    );

    /// Publishes a transition end event.
    ///
    /// # Arguments
    /// * `source` - The component that generated this event
    /// * `transition` - The transition that has ended
    async fn publish_transition_end(
        &self,
        source: &dyn Any,
        transition: &dyn StateMachineTransition<S, E>,
    );

    /// Publishes a transition event.
    ///
    /// # Arguments
    /// * `source` - The component that generated this event
    /// * `transition` - The transition that occurred
    async fn publish_transition(
        &self,
        source: &dyn Any,
        transition: &dyn StateMachineTransition<S, E>,
    );

    /// Publishes a state machine start event.
    ///
    /// # Arguments
    /// * `source` - The component that generated this event
    /// * `state_machine` - The state machine that started
    async fn publish_state_machine_start(
        &self,
        source: &dyn Any,
        state_machine: &dyn StateMachine<S, E>,
    );

    /// Publishes a state machine stop event.
    ///
    /// # Arguments
    /// * `source` - The component that generated this event
    /// * `state_machine` - The state machine that stopped
    async fn publish_state_machine_stop(
        &self,
        source: &dyn Any,
        state_machine: &dyn StateMachine<S, E>,
    );

    /// Publishes a state machine error event.
    ///
    /// # Arguments
    /// * `source` - The component that generated this event
    /// * `state_machine` - The state machine that encountered an error
    /// * `error` - The error that occurred
    async fn publish_state_machine_error(
        &self,
        source: &dyn Any,
        state_machine: &dyn StateMachine<S, E>,
        error: &(dyn std::error::Error + Send + Sync),
    );

    /// Publishes an extended state changed event.
    ///
    /// # Arguments
    /// * `source` - The component that generated this event
    /// * `key` - The key of the extended state variable that changed
    /// * `value` - The new value of the extended state variable
    async fn publish_extended_state_changed(&self, source: &dyn Any, key: &str, value: &AnyValue);
}
