//! A holder for a state reference.
//!
//! Pseudo states refer to the states of the machine they belong to, and the
//! machine is built bottom up: the holder breaks that cycle because it is
//! created empty and filled once the machine knows the state.

use std::sync::Arc;
use std::sync::RwLock;

use crate::state::StateMachineState;
use crate::support::lock_util::{read, write};

/// Utility type using the holder pattern to keep a state reference.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct StateHolder<S, E> {
    state: RwLock<Option<Arc<dyn StateMachineState<S, E>>>>,
}

impl<S, E> Default for StateHolder<S, E> {
    fn default() -> Self {
        Self {
            state: RwLock::new(None),
        }
    }
}

impl<S, E> StateHolder<S, E> {
    /// Creates an empty holder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a holder that already contains a state.
    ///
    /// # Arguments
    /// * `state` - the state to hold
    pub fn with_state(state: Arc<dyn StateMachineState<S, E>>) -> Self {
        Self {
            state: RwLock::new(Some(state)),
        }
    }

    /// Gets the held state.
    pub fn get_state(&self) -> Option<Arc<dyn StateMachineState<S, E>>> {
        read(&self.state).clone()
    }

    /// Sets the held state.
    ///
    /// # Arguments
    /// * `state` - the state to hold
    pub fn set_state(&self, state: Arc<dyn StateMachineState<S, E>>) {
        *write(&self.state) = Some(state);
    }
}
