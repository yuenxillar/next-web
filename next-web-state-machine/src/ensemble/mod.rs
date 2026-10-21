//! Coordination between several state machines.
/// A machine which is part of a distributed ensemble.
pub mod distributed_state_machine;
/// Listener of the ensemble lifecycle.
pub mod ensemble_listener;
/// The ensemble contract.
pub mod state_machine_ensemble;
/// Errors raised by an ensemble.
pub mod state_machine_ensemble_error;
/// Support object shared by the ensemble implementations.
pub mod state_machine_ensemble_object_support;
