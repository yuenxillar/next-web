use std::sync::Arc;

use crate::state_machine::{
    ensemble::ensemble_listener::EnsembleListener, state_machine_context::StateMachineContext,
    StateMachine,
};

/// `StateMachineEnsemble` is a contract between a `StateMachine` and
/// arbitrary ensemble of other `StateMachine`s.
///
/// This trait defines the interface for coordinating multiple state machines
/// in a distributed or clustered environment, providing mechanisms for
/// joining/leaving ensembles, leader election, and state synchronization.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
pub trait StateMachineEnsemble<S, E> {
    /// Request a join to a state machine ensemble. This method
    /// is a request to join an ensemble and doesn't guarantee
    /// a requester will eventually successfully join. Join operation
    /// needs to be used together with `EnsembleListener` and
    /// `EnsembleListener::state_machine_joined` is called with a
    /// `StateMachine` instance for successful join.
    ///
    /// # Arguments
    /// * `state_machine` - the state machine
    fn join(&mut self, state_machine: Arc<dyn StateMachine<S, E>>);

    /// Request a leave from an ensemble. This method is a request to
    /// leave an ensemble. After this method is called no further processing
    /// is done for a instance of `StateMachine`. Additionally
    /// `EnsembleListener::state_machine_left` is called when leave request
    /// is fully processed.
    ///
    /// # Arguments
    /// * `state_machine` - the state machine
    fn leave(&mut self, state_machine: &dyn StateMachine<S, E>);

    /// Adds the ensemble listener.
    ///
    /// # Arguments
    /// * `listener` - the listener
    fn add_ensemble_listener(&mut self, listener: Arc<dyn EnsembleListener<S, E>>);

    /// Removes the ensemble listener.
    ///
    /// # Arguments
    /// * `listener` - the listener
    fn remove_ensemble_listener(&mut self, listener: &dyn EnsembleListener<S, E>);

    /// Sets the state as a `StateMachineContext`.
    ///
    /// # Arguments
    /// * `context` - the state machine context
    fn set_state(&mut self, context: Arc<dyn StateMachineContext<S, E>>);

    /// Gets the state as a `StateMachineContext`.
    ///
    /// # Returns
    /// The state machine context
    fn get_state(&self) -> Option<&dyn StateMachineContext<S, E>>;

    /// Gets the ensemble leader. If returned machine
    /// is `None` it indicates that this ensemble
    /// doesn't know any leader.
    ///
    /// # Returns
    /// The ensemble leader, if any
    fn get_leader(&self) -> Option<&dyn StateMachine<S, E>>;
}
