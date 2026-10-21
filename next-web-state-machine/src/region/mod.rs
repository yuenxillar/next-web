//! Regions are the executable part of a state machine.
//!
//! A state machine is a region, and a composite state contains regions. The
//! trait is the common surface used by the runtime for both.

/// The policy which decides whether events are processed sequentially.
pub mod region_execution_policy;

use std::sync::Arc;

use next_web_core::{async_trait, error::BoxError, messaging::Message};
use uuid::Uuid;

use crate::listener::state_machine_listener::StateMachineListener;
use crate::state::StateMachineState;
use crate::state_machine_event_result::StateMachineEventResult;
use crate::transition::StateMachineTransition;

/// The executable part of a machine.
#[async_trait]
pub trait Region<S, E>: Send + Sync
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    /// Gets the region and state machine id.
    ///
    /// This identifier is provided for users disposal and can be set from
    /// various ways to build a machine.
    fn id(&self) -> &str;

    /// Gets the unique id of the machine behind this region.
    fn get_uuid(&self) -> Uuid;

    /// Starts the region.
    async fn start(&self) -> Result<(), BoxError>;

    /// Stops the region.
    async fn stop(&self) -> Result<(), BoxError>;

    /// Sends an event to the region.
    ///
    /// The events of one machine are processed one at a time: a caller which
    /// sends from several tasks waits until the running event finished.
    ///
    /// A state action must not await a send to its own machine, because the
    /// event which runs the action still holds the processing slot; spawn the
    /// follow-up event instead.
    ///
    /// # Arguments
    /// * `event` - the event to send
    async fn send_event(
        &self,
        event: Arc<dyn Message<E>>,
    ) -> Result<Arc<dyn StateMachineEventResult<S, E>>, BoxError>;

    /// Sends multiple events to the region.
    ///
    /// # Arguments
    /// * `events` - the events to send
    async fn send_events(
        &self,
        events: Vec<Arc<dyn Message<E>>>,
    ) -> Result<Vec<Arc<dyn StateMachineEventResult<S, E>>>, BoxError>;

    /// Gets the current state.
    fn state(&self) -> Option<Arc<dyn StateMachineState<S, E>>>;

    /// Gets the current states.
    fn states(&self) -> Vec<Arc<dyn StateMachineState<S, E>>>;

    /// Gets the transitions of this region.
    fn transitions(&self) -> Vec<&dyn StateMachineTransition<S, E>>;

    /// Checks if the region is complete.
    ///
    /// A region is complete when it reached its end state and no further event
    /// processing is happening.
    fn is_complete(&self) -> bool;

    /// Sets the event forwarded to the initial transition of this region.
    ///
    /// # Arguments
    /// * `message` - the message to forward
    fn set_forwarded_initial_event(&self, message: Arc<dyn Message<E>>) {
        let _ = message;
    }

    /// Enables or disables the initial transition of this region.
    ///
    /// # Arguments
    /// * `enabled` - whether the initial transition runs on start
    fn set_initial_enabled(&self, enabled: bool) {
        let _ = enabled;
    }

    /// Sets the current state of this region.
    ///
    /// A state machine delegates here when a transition of its region targets a
    /// state that belongs to the region instead of to itself.
    ///
    /// # Arguments
    /// * `state` - the state to switch to
    /// * `message` - the message which caused the change
    /// * `transition` - the transition behind the change
    /// * `exit` - whether the current state of the region must be exited first
    async fn set_current_state(
        &self,
        state: Arc<dyn StateMachineState<S, E>>,
        message: Option<Arc<dyn Message<E>>>,
        transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
        exit: bool,
    ) -> Result<(), BoxError> {
        let _ = (state, message, transition, exit);
        Ok(())
    }

    /// Adds a state listener to this region.
    ///
    /// # Arguments
    /// * `listener` - the listener
    fn add_state_listener(
        &self,
        listener: Arc<dyn StateMachineListener<S, E>>,
    ) -> Result<(), BoxError>;

    /// Removes a state listener from this region.
    ///
    /// # Arguments
    /// * `listener` - the listener
    fn remove_state_listener(
        &self,
        listener: Arc<dyn StateMachineListener<S, E>>,
    ) -> Result<(), BoxError>;
}
