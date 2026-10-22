//! Circuit breaking: the breaker state machine and its registry.

/// The breaker state machine.
pub mod breaker_controller;
pub mod circuit_breaker_error;
pub(crate) mod circuit_breaker_service;
pub(crate) mod circuit_breaker_service_manager;
pub mod circuit_state;

/// Short alias for the breaker state machine.
pub use breaker_controller as circuit_breaker_controller;
