//! The entry point pseudo state.

use std::sync::Arc;

use futures::future::BoxFuture;
use next_web_core::error::BoxError;

use crate::state::StateMachineState;
use crate::state::pseudo_state::PseudoState;
use crate::state::pseudo_state_context::PseudoStateContext;
use crate::state::pseudo_state_kind::PseudoStateKind;
use crate::state::pseudo_state_listener::PseudoStateListener;
use crate::state_context::StateContext;

/// Entrypoint implementation of a [`PseudoState`].
///
/// Entering it continues with the state it points to.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct EntryPseudoState<S, E> {
    state: Arc<dyn StateMachineState<S, E>>,
}

impl<S, E> EntryPseudoState<S, E> {
    /// Creates a new entry pseudo state.
    ///
    /// # Arguments
    /// * `state` - the state this entry point refers to
    pub fn new(state: Arc<dyn StateMachineState<S, E>>) -> Self {
        Self { state }
    }
}

impl<S, E> PseudoState<S, E> for EntryPseudoState<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn get_kind(&self) -> PseudoStateKind {
        PseudoStateKind::Entry
    }

    fn entry<'a>(
        &'a self,
        _context: &'a dyn StateContext<S, E>,
    ) -> BoxFuture<'a, Option<Arc<dyn StateMachineState<S, E>>>> {
        Box::pin(async move { Some(self.state.clone()) })
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
