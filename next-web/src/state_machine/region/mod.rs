pub mod region_execution_policy;

use std::sync::Arc;

use next_web_core::{async_trait, error::BoxError, traits::message::Message};

use crate::state_machine::{
    listener::state_machine_listener::StateMachineListener, state::StateMachineState,
    state_machine_event_result::StateMachineEventResult, transition::StateMachineTransition,
};

#[async_trait]
pub trait Region<S, E> {
    /// Gets the region and state machine id.
    /// This identifier is provided for users disposal and can be set from a various ways to build a machines
    fn id(&self) -> &str;

    /// Start the region.
    async fn start(&self) -> Result<(), BoxError>;

    /// Stop the region.
    async fn stop(&self) -> Result<(), BoxError>;

    /// Send an event to the region.
    async fn send_event(
        &self,
        event: Box<dyn Message<E>>,
    ) -> Result<Box<dyn StateMachineEventResult<S, E>>, BoxError>;

    /// Send multiple events to the region.
    async fn send_events(
        &self,
        events: Vec<Box<dyn Message<E>>>,
    ) -> Result<Box<dyn StateMachineEventResult<S, E>>, BoxError>;

    /// Gets the current State.
    fn state(&self) -> Option<&dyn StateMachineState<S, E>>;

    /// Gets the current States.
    fn states(&self) -> Vec<&dyn StateMachineState<S, E>>;

    /// Gets a Transitions for this region.
    fn transitions(&self) -> Vec<&dyn StateMachineTransition<S, E>>;

    /// Checks if region complete.
    /// Region is considered to be completed if it has reached its end state and no further event processing is happening.
    fn is_complete(&self) -> bool;

    /// Adds the state listener.
    fn add_state_listener(
        &self,
        listener: Arc<dyn StateMachineListener<S, E>>,
    ) -> Result<(), BoxError>;

    /// Removes the state listener.
    fn remove_state_listener(
        &self,
        listener: &dyn StateMachineListener<S, E>,
    ) -> Result<(), BoxError>;
}
