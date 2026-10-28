//! Listener registration contracts for the RocketMQ starter.

/// Stores the shared listener metadata before a concrete transport binds it.
pub mod base_rocketmq_listener_registration;

/// Wraps the base registration with the default auto-configuration behavior.
pub mod default_rocketmq_listener_registration;

/// Collects listener registrations during auto-configuration.
pub mod default_rocketmq_listener_registry;

/// Callback contract implemented by application modules to declare listeners.
pub mod rocketmq_configurer;

/// Fluent registration contract used when declaring RocketMQ listeners.
pub mod rocketmq_listener_registration;

/// Registry abstraction used to collect listener declarations.
pub mod rocketmq_listener_registry;
