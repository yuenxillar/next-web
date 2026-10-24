//! Persistence of a machine state.
/// Persists and restores a machine through a state machine persist.
pub mod default_state_machine_persister;
/// Writes the machine state whenever it changes.
pub mod persisting_state_machine_interceptor;
/// The persister contract.
pub mod state_machine_persister;
/// A persister which also supplies an interceptor.
pub mod state_machine_runtime_persister;
