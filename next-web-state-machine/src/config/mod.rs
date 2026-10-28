//! Configuration of a state machine.
//!
//! The configuration layer turns a model into a running machine: fluent builders
//! collect the data, configurers fill it in, and a factory builds the runtime.

/// Transition actions, state action policies and action listeners.
pub mod action;
/// The factory which translates a model into a running machine.
pub mod base_state_machine_factory;
/// Fluent builders for configuration, states and transitions.
pub mod builders;
/// Building blocks shared by the builders, such as configurer adapters.
pub mod common;
/// The configuration model of a state machine.
pub mod configuration;
/// The contract of a configurer which returns to its parent builder.
pub mod configurer_builder;
/// Configurers for the individual parts of a machine.
pub mod configurers;
/// The default factory used by the auto configuration.
pub mod default_state_machine_factory;
/// Guards which gate a transition.
pub mod guard;
/// The data model of states and transitions.
pub mod model;
/// The built configuration of a machine.
pub mod state_machine_config;
/// Legacy configuration snapshot, kept for the old builder path.
pub mod state_machine_configure;
/// Adapter which applies a configurer to a builder.
pub mod state_machine_configurer_adapter;
/// The factory contract for building machines.
pub mod state_machine_factory;
/// Legacy state configuration snapshot, kept for the old builder path.
pub mod state_machine_state_configure;
/// Legacy transition configuration snapshot, kept for the old builder path.
pub mod state_machine_transition_configure;
