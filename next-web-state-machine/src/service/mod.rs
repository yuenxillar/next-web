//! The service layer of a state machine.
//!
//! A service owns the machines of an application: it builds them on demand,
//! keeps one instance per identifier alive, persists and restores them and stops
//! them when the application releases them.

/// The service contract.
pub mod state_machine_service;
/// The default service, which keeps the acquired machines in memory.
pub mod default_state_machine_service;
