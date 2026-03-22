use next_web_core::{error::BoxError, traits::message::Message};

use crate::state_machine::{
    state::StateMachineState, state_context::StateContext, transition::StateMachineTransition,
    StateMachine,
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
    /// Called before message is sent to processing.
    /// returning Err will skip the message.
    ///
    /// # Arguments
    /// * `message` - the message
    /// * `state_machine` - the state machine
    ///
    /// # Returns
    /// The intercepted message, or `None` to skip the message
    fn pre_event(
        &self,
        message: &mut dyn Message<E>,
        state_machine: &dyn StateMachine<S, E>,
    ) -> Result<(), BoxError>;

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
        message: &dyn Message<E>,
        transition: &dyn StateMachineTransition<S, E>,
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
        message: &dyn Message<E>,
        transition: &dyn StateMachineTransition<S, E>,
        state_machine: &dyn StateMachine<S, E>,
        root_state_machine: &dyn StateMachine<S, E>,
    );

    /// Called prior of a start of a transition. Returning
    /// `None` from this method will break the transition
    /// chain.
    ///
    /// # Arguments
    /// * `state_context` - the state context
    ///
    /// # Returns
    /// The state context, or `None` to break the transition chain
    fn pre_transition(&self, state_context: &dyn StateContext<S, E>) -> Result<(), BoxError>;

    /// Called after of a transition if transition happened.
    ///
    /// # Arguments
    /// * `state_context` - the state context
    ///
    /// # Returns
    /// The state context
    fn post_transition(&self, state_context: &dyn StateContext<S, E>) -> Result<(), BoxError>;

    /// State when state machine is about to enter error it can't recover.
    ///
    /// # Arguments
    /// * `state_machine` - the state machine
    /// * `exception` - the exception
    ///
    /// # Returns
    /// The exception, potentially modified
    fn state_machine_error(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        error: Box<dyn std::error::Error>,
    ) -> Box<dyn std::error::Error>;
}
