//! State machine modeling and runtime support.
//!
//! A machine is described by a model, built by a factory from the configuration
//! layer, and driven by the runtime in [`state`], [`transition`] and [`support`].

// The configuration layer mirrors the Java API, whose generic parameters become
// deeply nested trait object types and whose builder methods take the same long
// parameter lists. Grouping them into aliases or parameter structs would hide
// that one-to-one mapping, so these two lints are allowed crate wide.
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

/// Access to the regions of a machine.
pub mod access;
/// Actions and action policies.
pub mod action;
/// Wiring of the default configuration.
pub mod autoconfigure;
/// The configuration layer: models, builders, configurers and factories.
pub mod config;
/// Coordination between several machines.
pub mod ensemble;
/// Events published by a machine.
pub mod event;
/// The extended state of a machine.
pub mod extended_state;
/// Handlers which react to the transitions of a machine.
pub mod handler;
/// Listeners of a machine.
pub mod listener;
/// Monitors which measure transitions and actions.
pub mod monitor;
/// Persistence of a machine state.
pub mod persist;
/// Regions, the executable part of a machine.
pub mod region;
/// Security rules and their enforcement.
pub mod security;
/// The service layer which owns the machines of an application.
pub mod service;
/// The state model.
pub mod state;
/// The context passed to actions, guards and listeners.
pub mod state_context;
/// The captured context of a machine, used by persistence.
pub mod state_machine_context;
/// The result of sending an event to a machine.
pub mod state_machine_event_result;
/// The error type of the crate.
pub mod state_machine_exception;
/// Serialization of a machine context.
pub mod state_machine_persist;
/// Constants shared by the machine components.
pub mod state_machine_system_constants;
/// The runtime support: machine, executor, contexts and utilities.
pub mod support;
/// The transition model.
pub mod transition;
/// Triggers which fire a transition.
pub mod trigger;

mod state_machine;

pub use state_machine::{BoxedStateAction, BoxedStateGuard, StateMachine, StateMachineId};
pub use state_machine_exception::{
    StateMachineErrorKind, StateMachineException, execution_error, state_machine_error,
};
