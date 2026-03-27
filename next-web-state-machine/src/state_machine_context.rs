use std::collections::HashMap;

use crate::extended_state::ExtendedState;

/// `StateMachineContext` represents a current state of a state machine.
///
/// This trait defines the interface for capturing and restoring the complete
/// state of a state machine, including hierarchical state information,
/// history, extended state, and event data.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
pub trait StateMachineContext<S, E> {
    /// Gets the machine id.
    ///
    /// # Returns
    /// The machine id
    fn id(&self) -> &str;

    /// Gets the child contexts if any.
    ///
    /// # Returns
    /// The child contexts
    fn childs(&self) -> &[Box<dyn StateMachineContext<S, E>>];

    /// Gets the child context references if any.
    ///
    /// # Returns
    /// The child context references
    fn child_references(&self) -> &[String];

    /// Gets the state.
    ///
    /// # Returns
    /// The current state
    fn state(&self) -> &S;

    /// Gets the event.
    ///
    /// # Returns
    /// The current event, if any
    fn event(&self) -> Option<&E>;

    /// Gets the history state mappings.
    ///
    /// # Returns
    /// The history state mappings
    fn history_states(&self) -> &HashMap<S, S>;

    /// Gets the event headers.
    ///
    /// # Returns
    /// The event headers
    fn event_headers(&self) -> &HashMap<String, String>;

    /// Gets the extended state.
    ///
    /// # Returns
    /// The extended state
    fn extended_state(&self) -> &dyn ExtendedState;
}
