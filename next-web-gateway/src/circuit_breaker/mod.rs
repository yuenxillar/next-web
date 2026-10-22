//! Circuit breaking: controller, per-route service and fallback plumbing.

/// The breaker state machine. The module name is kept for compatibility.
#[path = "breaker_controller.rs"]
pub mod circuit_breaker_controller;

/// The breaker state machine, under its short name.
pub use circuit_breaker_controller as breaker_controller;

pub mod circuit_breaker_error;
pub(crate) mod circuit_breaker_service;
pub(crate) mod circuit_breaker_service_manager;
pub mod circuit_state;
pub mod default_fallback_provider;
pub mod fallback_error;
pub mod fallback_provider;
