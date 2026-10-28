//! The pseudo state contract.

use std::any::Any;
use std::sync::Arc;

use futures::future::BoxFuture;
use next_web_core::error::BoxError;

use crate::state::pseudo_state_kind::PseudoStateKind;
use crate::state::pseudo_state_listener::PseudoStateListener;
use crate::state::StateMachineState;
use crate::state_context::StateContext;

/// Trait for pseudo state functionality.
///
/// A pseudo state is attached to a regular state and intercepts the entry and
/// exit of it, which is how initial, end, choice, junction, fork, join, entry,
/// exit and history states are implemented.
pub trait PseudoState<S, E>: Send + Sync + Any + 'static {
    /// Gets the kind of this pseudo state.
    fn get_kind(&self) -> PseudoStateKind;

    /// Called when entering this pseudo state.
    ///
    /// # Arguments
    /// * `context` - the state context
    ///
    /// # Returns
    /// The state to continue with, or `None` to stay in the current state
    fn entry<'a>(
        &'a self,
        context: &'a dyn StateContext<S, E>,
    ) -> BoxFuture<'a, Option<Arc<dyn StateMachineState<S, E>>>>;

    /// Called when exiting this pseudo state.
    ///
    /// # Arguments
    /// * `context` - the state context
    fn exit<'a>(
        &'a self,
        context: &'a dyn StateContext<S, E>,
    ) -> BoxFuture<'a, Result<(), BoxError>>;

    /// Adds a pseudo state listener.
    ///
    /// # Arguments
    /// * `listener` - the listener to add
    fn add_pseudo_state_listener(&self, listener: Arc<dyn PseudoStateListener<S, E>>);

    /// Sets pseudo state listeners, replacing any existing ones.
    ///
    /// # Arguments
    /// * `listeners` - the new listeners
    fn set_pseudo_state_listeners(&self, listeners: Vec<Arc<dyn PseudoStateListener<S, E>>>);

    /// Notifies the pseudo state listeners of a new context.
    ///
    /// # Arguments
    /// * `context` - the pseudo state context
    fn notify_context(
        &self,
        context: &dyn crate::state::pseudo_state_context::PseudoStateContext<S, E>,
    );
}
