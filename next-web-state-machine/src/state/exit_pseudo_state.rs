//! The exit point pseudo state.

use std::sync::Arc;

use futures::future::BoxFuture;
use next_web_core::error::BoxError;

use crate::state::StateMachineState;
use crate::state::pseudo_state::PseudoState;
use crate::state::pseudo_state_context::PseudoStateContext;
use crate::state::pseudo_state_kind::PseudoStateKind;
use crate::state::pseudo_state_listener::PseudoStateListener;
use crate::state::state_holder::StateHolder;
use crate::state_context::StateContext;

/// Exitpoint implementation of a [`PseudoState`].
///
/// Entering it continues with the state kept in its holder.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct ExitPseudoState<S, E> {
    state: StateHolder<S, E>,
}

impl<S, E> ExitPseudoState<S, E> {
    /// Creates a new exit pseudo state.
    ///
    /// # Arguments
    /// * `state` - the holder of the state this exit point refers to
    pub fn new(state: StateHolder<S, E>) -> Self {
        Self { state }
    }

    /// Gets the state holder.
    pub fn get_state_holder(&self) -> &StateHolder<S, E> {
        &self.state
    }
}

impl<S, E> PseudoState<S, E> for ExitPseudoState<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn get_kind(&self) -> PseudoStateKind {
        PseudoStateKind::Exit
    }

    fn entry<'a>(
        &'a self,
        _context: &'a dyn StateContext<S, E>,
    ) -> BoxFuture<'a, Option<Arc<dyn StateMachineState<S, E>>>> {
        Box::pin(async move { self.state.get_state() })
    }

    fn exit<'a>(
        &'a self,
        _context: &'a dyn StateContext<S, E>,
    ) -> BoxFuture<'a, Result<(), BoxError>> {
        Box::pin(async move { Ok(()) })
    }

    fn add_pseudo_state_listener(&self, _listener: Arc<dyn PseudoStateListener<S, E>>) {}

    fn set_pseudo_state_listeners(&self, _listeners: Vec<Arc<dyn PseudoStateListener<S, E>>>) {}

    fn notify_context(&self, _context: &dyn PseudoStateContext<S, E>) {}
}
