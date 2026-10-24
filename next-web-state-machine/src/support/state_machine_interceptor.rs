use std::sync::Arc;

use next_web_core::{error::BoxError, messaging::Message};

use crate::{
    StateMachine, state::StateMachineState, state_context::StateContext,
    transition::StateMachineTransition,
};

/// Interface which can be registered with a state machine and can be used
/// to intercept and break a state change chain.
///
/// This trait defines methods for intercepting various points in the state
/// machine lifecycle, allowing for custom behavior, validation, logging,
/// or modification of the state transition process.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
pub trait StateMachineInterceptor<S, E>
where
    Self: Send + Sync,
{
    /// Called before a message is sent to processing.
    ///
    /// # Arguments
    /// * `message` - the message
    /// * `state_machine` - the state machine
    ///
    /// # Returns
    /// The message to continue with, or an error to skip the message. The
    /// returned message allows an interceptor to replace the payload, which is
    /// what the Java implementation does by returning a new `Message`.
    fn pre_event(
        &self,
        message: Arc<dyn Message<E>>,
        state_machine: &dyn StateMachine<S, E>,
    ) -> Result<Arc<dyn Message<E>>, BoxError>;

    /// Called prior of a state change. Returning an error
    /// from this method will stop a state change logic.
    ///
    /// # Arguments
    /// * `state` - the state
    /// * `message` - the message
    /// * `transition` - the transition
    /// * `state_machine` - the state machine
    /// * `root_state_machine` - the root state machine
    ///
    /// # Returns
    /// `Ok(())` to continue, `Err(error)` to stop the state change
    fn pre_state_change(
        &self,
        state: &dyn StateMachineState<S, E>,
        message: Option<&dyn Message<E>>,
        transition: Option<&dyn StateMachineTransition<S, E>>,
        state_machine: &dyn StateMachine<S, E>,
        root_state_machine: &dyn StateMachine<S, E>,
    ) -> Result<(), BoxError>;

    /// Called after a state change.
    ///
    /// # Arguments
    /// * `state` - the state
    /// * `message` - the message
    /// * `transition` - the transition
    /// * `state_machine` - the state machine
    /// * `root_state_machine` - the root state machine
    fn post_state_change(
        &self,
        state: &dyn StateMachineState<S, E>,
        message: Option<&dyn Message<E>>,
        transition: Option<&dyn StateMachineTransition<S, E>>,
        state_machine: &dyn StateMachine<S, E>,
        root_state_machine: &dyn StateMachine<S, E>,
    );

    /// Called prior of a start of a transition. Returning an
    /// error from this method will break the transition chain.
    ///
    /// # Arguments
    /// * `state_context` - the state context
    ///
    /// # Returns
    /// `Ok(())` to continue, `Err(error)` to break the transition chain
    fn pre_transition(&self, state_context: &dyn StateContext<S, E>) -> Result<(), BoxError>;

    /// Called after a transition if the transition happened.
    ///
    /// # Arguments
    /// * `state_context` - the state context
    fn post_transition(&self, state_context: &dyn StateContext<S, E>) -> Result<(), BoxError>;

    /// Called when the state machine is about to enter an error it cannot
    /// recover from.
    ///
    /// # Arguments
    /// * `state_machine` - the state machine
    /// * `error` - the error
    ///
    /// # Returns
    /// The error, potentially modified. `None` suppresses the error.
    fn state_machine_error(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        error: BoxError,
    ) -> Option<BoxError>;
}
