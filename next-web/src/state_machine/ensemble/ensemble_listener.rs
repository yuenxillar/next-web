use crate::state_machine::{
    ensemble::state_machine_ensemble_error::StateMachineEnsembleError,
    state_machine_context::StateMachineContext, StateMachine,
};

/// `EnsembleListener` for various ensemble events.
///
/// This trait defines callbacks for monitoring events in a state machine
/// ensemble, including joining/leaving, state changes, leadership changes,
/// and error conditions.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
pub trait EnsembleListener<S, E> {
    /// Called when state machine joined an ensemble. This callback
    /// is guaranteed to be called for a `StateMachine` who
    /// requested a join. User of this listener should check that a
    /// `StateMachine` is the one interested of. Implementation
    /// may choose to notify other `StateMachine` joins if it is
    /// able to do so. This may be called multiple time in case ensemble
    /// has made a choice to leave machine due to ensemble errors.
    ///
    /// # Arguments
    /// * `state_machine` - the state machine
    /// * `context` - the state machine context
    fn state_machine_joined(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        context: &dyn StateMachineContext<S, E>,
    );

    /// Called when state machine left an ensemble. This callback
    /// is guaranteed to be called for a `StateMachine` who
    /// requested a leave. User of this listener should check that a
    /// `StateMachine` is the one interested of. Implementation
    /// may choose to notify other `StateMachine` leaves if it is
    /// able to do so.
    ///
    /// # Arguments
    /// * `state_machine` - the state machine
    /// * `context` - the state machine context
    fn state_machine_left(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        context: &dyn StateMachineContext<S, E>,
    );

    /// Called when ensemble is discovering a state change.
    ///
    /// # Arguments
    /// * `context` - the state machine context
    fn state_changed(&self, context: &dyn StateMachineContext<S, E>);

    /// Called when `StateMachineEnsemble` resulted an error.
    ///
    /// # Arguments
    /// * `error` - the error
    fn ensemble_error(&self, error: &StateMachineEnsembleError);

    /// Called when a state machine is granted a leader role
    /// in an ensemble.
    ///
    /// # Arguments
    /// * `state_machine` - the state machine
    fn ensemble_leader_granted(&self, state_machine: &dyn StateMachine<S, E>);

    /// Called when a state machine is revoked from a leader role
    /// in an ensemble.
    ///
    /// # Arguments
    /// * `state_machine` - the state machine
    fn ensemble_leader_revoked(&self, state_machine: &dyn StateMachine<S, E>);
}
