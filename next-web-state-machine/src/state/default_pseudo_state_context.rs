//! Default implementation of a pseudo state context.

use std::sync::Arc;

use crate::state::pseudo_state::PseudoState;
use crate::state::pseudo_state_context::{PseudoAction, PseudoStateContext};

/// Default [`PseudoStateContext`] carrying the pseudo state which raised the
/// notification and the action that caused it.
pub struct DefaultPseudoStateContext<S, E> {
    pseudo_state: Option<Arc<dyn PseudoState<S, E>>>,
    pseudo_action: PseudoAction,
}

impl<S, E> DefaultPseudoStateContext<S, E> {
    /// Creates a new context for a pseudo state.
    ///
    /// # Arguments
    /// * `pseudo_state` - the pseudo state which raises the notification
    /// * `pseudo_action` - the action which caused the notification
    pub fn new(pseudo_state: Arc<dyn PseudoState<S, E>>, pseudo_action: PseudoAction) -> Self {
        Self {
            pseudo_state: Some(pseudo_state),
            pseudo_action,
        }
    }

    /// Creates a context which only carries the action.
    ///
    /// # Arguments
    /// * `pseudo_action` - the action which caused the notification
    pub fn from_action(pseudo_action: PseudoAction) -> Self {
        Self {
            pseudo_state: None,
            pseudo_action,
        }
    }
}

impl<S, E> PseudoStateContext<S, E> for DefaultPseudoStateContext<S, E> {
    fn get_pseudo_state(&self) -> Option<&dyn PseudoState<S, E>> {
        self.pseudo_state.as_deref()
    }

    fn get_pseudo_action(&self) -> PseudoAction {
        self.pseudo_action
    }
}
