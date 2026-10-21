//! Listeners of a state machine.
/// A listener which dispatches to a set of listeners.
pub mod base_composite_listener;
/// The composite listener of a state machine.
pub mod composite_state_machine_listener;
/// The default listener composite.
pub mod default_composite_listener;
/// A composite which keeps its items in order.
pub mod ordered_composite;
/// The listener contract.
pub mod state_machine_listener;
/// An adapter which implements every callback as a no-op.
pub mod state_machine_listener_adapter;
