//! The fork pseudo state.

use std::sync::Arc;

use futures::future::BoxFuture;
use next_web_core::error::BoxError;

use crate::state::StateMachineState;
use crate::state::composite_pseudo_state_listener::CompositePseudoStateListener;
use crate::state::pseudo_state::PseudoState;
use crate::state::pseudo_state_context::PseudoStateContext;
use crate::state::pseudo_state_kind::PseudoStateKind;
use crate::state::pseudo_state_listener::PseudoStateListener;
use crate::state_context::StateContext;

/// Fork implementation of a [`PseudoState`].
///
/// The machine fans out to every state returned by [`Self::get_forks`] when it
/// enters this pseudo state.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct ForkPseudoState<S, E> {
    forks: Vec<Arc<dyn StateMachineState<S, E>>>,
    listeners: CompositePseudoStateListener<S, E>,
}

impl<S, E> ForkPseudoState<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    /// Creates a new fork pseudo state.
    ///
    /// # Arguments
    /// * `forks` - the states the machine forks into
    pub fn new(forks: Vec<Arc<dyn StateMachineState<S, E>>>) -> Self {
        Self {
            forks,
            listeners: CompositePseudoStateListener::new(),
        }
    }

    /// Gets the states this pseudo state forks into.
    pub fn get_forks(&self) -> &[Arc<dyn StateMachineState<S, E>>] {
        &self.forks
    }
}

impl<S, E> PseudoState<S, E> for ForkPseudoState<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn get_kind(&self) -> PseudoStateKind {
        PseudoStateKind::Fork
    }

    fn entry<'a>(
        &'a self,
        _context: &'a dyn StateContext<S, E>,
    ) -> BoxFuture<'a, Option<Arc<dyn StateMachineState<S, E>>>> {
        Box::pin(async move { None })
    }

    fn exit<'a>(
        &'a self,
        _context: &'a dyn StateContext<S, E>,
    ) -> BoxFuture<'a, Result<(), BoxError>> {
        Box::pin(async move { Ok(()) })
    }

    fn add_pseudo_state_listener(&self, listener: Arc<dyn PseudoStateListener<S, E>>) {
        self.listeners.register(listener);
    }

    fn set_pseudo_state_listeners(&self, listeners: Vec<Arc<dyn PseudoStateListener<S, E>>>) {
        self.listeners.set_listeners(listeners);
    }

    fn notify_context(&self, context: &dyn PseudoStateContext<S, E>) {
        self.listeners.on_context(context);
    }
}
