//! Core RocketMQ abstractions shared by auto-configuration, producers, and
//! listeners.

/// Declares listener endpoint metadata and message model options.
pub mod endpoint;

/// Defines the listener callback contract.
pub mod listener;

/// Defines transport-neutral message structures exposed by the starter.
pub mod message;
