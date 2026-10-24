//! The runtime support of a state machine.
/// The machine runtime which drives states, transitions and events.
pub mod base_state_machine;
/// The default extended state.
pub mod default_extended_state;
/// The default context passed to actions, guards and listeners.
pub mod default_state_context;
/// The default captured context, used by persistence.
pub mod default_state_machine_context;
/// The default executor which processes events and triggers.
pub mod default_state_machine_executor;
/// Lifecycle hooks shared by the machine components.
pub mod lifecycle_object_support;
/// Lock helpers which never panic on a poisoned lock.
pub mod lock_util;
/// An observable map, used by the extended state.
pub mod observable_map;
/// An ordered composite item, used by the composites.
pub mod ordered_composite_item;
/// The executor contract.
pub mod state_machine_executor;
/// The interceptor contract.
pub mod state_machine_interceptor;
/// The interceptor chain.
pub mod state_machine_interceptor_list;
/// Support object which publishes notifications and keeps the listeners.
pub mod state_machine_object_support;
/// Utilities shared by the runtime.
pub mod state_machine_utils;
/// Compares transitions so the machine picks a deterministic one.
pub mod transition_comparator;
