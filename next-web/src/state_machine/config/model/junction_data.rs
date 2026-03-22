use std::sync::Arc;

use crate::state_machine::config::{action::StateMachineAction, guard::StateMachineGuard};

/// A simple data object keeping junction related configs in a same place.
///
/// This struct encapsulates configuration data for a junction (pseudo-state) in a
/// state machine, which represents a dynamic conditional branch point with multiple
/// possible outgoing transitions evaluated in order.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
#[derive(Clone)]
pub struct JunctionData<S, E> {
    /// Source state of the junction
    source: S,
    /// Target state of the junction (if guard condition is satisfied)
    target: S,
    /// Guard condition that determines if this junction branch should be taken
    guard: Arc<dyn StateMachineGuard<S, E>>,
    /// Actions to execute when this junction branch is taken
    actions: Vec<Arc<dyn StateMachineAction<S, E>>>,
}

impl<S, E> JunctionData<S, E>
where
    S: Clone + Eq,
    E: Clone,
    S: Send + Sync,
    E: Send + Sync,
{
    /// Creates a new junction data with source, target, and guard.
    ///
    /// # Arguments
    /// * `source` - Source state
    /// * `target` - Target state
    /// * `guard` - Guard condition
    ///
    /// # Returns
    /// A new `JunctionData` instance
    pub fn new(source: S, target: S, guard: Arc<dyn StateMachineGuard<S, E>>) -> Self {
        Self {
            source,
            target,
            guard,
            actions: Vec::new(),
        }
    }

    /// Creates a new junction data with source, target, guard, and actions.
    ///
    /// # Arguments
    /// * `source` - Source state
    /// * `target` - Target state
    /// * `guard` - Guard condition
    /// * `actions` - Actions to execute
    ///
    /// # Returns
    /// A new `JunctionData` instance
    pub fn with_actions(
        source: S,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
        actions: Vec<Arc<dyn StateMachineAction<S, E>>>,
    ) -> Self {
        Self {
            source,
            target,
            guard,
            actions,
        }
    }

    /// Gets the source state.
    ///
    /// # Returns
    /// The source state
    pub fn source(&self) -> &S {
        &self.source
    }

    /// Gets the target state.
    ///
    /// # Returns
    /// The target state
    pub fn target(&self) -> &S {
        &self.target
    }

    /// Gets the guard condition.
    ///
    /// # Returns
    /// The guard condition
    pub fn guard(&self) -> &dyn StateMachineGuard<S, E> {
        self.guard.as_ref()
    }

    /// Gets the actions to execute.
    ///
    /// # Returns
    /// A slice of action functions
    pub fn actions(&self) -> &[Arc<dyn StateMachineAction<S, E>>] {
        &self.actions
    }

    /// Checks if the junction branch has actions.
    ///
    /// # Returns
    /// `true` if the junction branch has actions, `false` otherwise
    pub fn has_actions(&self) -> bool {
        !self.actions.is_empty()
    }
}
