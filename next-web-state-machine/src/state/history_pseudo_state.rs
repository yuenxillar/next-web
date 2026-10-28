//! The shallow and deep history pseudo states.

use std::sync::Arc;
use std::sync::RwLock;

use futures::future::BoxFuture;
use next_web_core::error::BoxError;

use crate::state::StateMachineState;
use crate::state::composite_pseudo_state_listener::CompositePseudoStateListener;
use crate::state::pseudo_state::PseudoState;
use crate::state::pseudo_state_context::PseudoStateContext;
use crate::state::pseudo_state_kind::PseudoStateKind;
use crate::state::pseudo_state_listener::PseudoStateListener;
use crate::state::state_holder::StateHolder;
use crate::state_context::StateContext;
use crate::support::lock_util::{read, write};

/// History implementation of a [`PseudoState`].
///
/// It remembers the last state of its containing state and returns to it,
/// unless there is no history or the remembered state is an end state; in that
/// case the default state, or the containing state itself, is entered.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct HistoryPseudoState<S, E> {
    kind: PseudoStateKind,
    default_state: StateHolder<S, E>,
    containing_state: StateHolder<S, E>,
    state: RwLock<Option<Arc<dyn StateMachineState<S, E>>>>,
    listeners: CompositePseudoStateListener<S, E>,
}

impl<S, E> HistoryPseudoState<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    /// Creates a new history pseudo state.
    ///
    /// # Arguments
    /// * `kind` - either [`PseudoStateKind::HistoryShallow`] or [`PseudoStateKind::HistoryDeep`]
    /// * `default_state` - the state entered when no history is recorded
    /// * `containing_state` - the parent state of this history state
    pub fn new(
        kind: PseudoStateKind,
        default_state: StateHolder<S, E>,
        containing_state: StateHolder<S, E>,
    ) -> Self {
        Self {
            kind,
            default_state,
            containing_state,
            state: RwLock::new(None),
            listeners: CompositePseudoStateListener::new(),
        }
    }

    /// Sets the currently recorded state.
    ///
    /// # Arguments
    /// * `state` - the state to remember
    pub fn set_state(&self, state: Arc<dyn StateMachineState<S, E>>) {
        *write(&self.state) = Some(state);
    }

    /// Gets the currently recorded state.
    pub fn get_state(&self) -> Option<Arc<dyn StateMachineState<S, E>>> {
        read(&self.state).clone()
    }

    /// Gets the containing state holder.
    pub fn get_containing_state(&self) -> &StateHolder<S, E> {
        &self.containing_state
    }

    /// Gets the default state holder.
    pub fn get_default_state(&self) -> &StateHolder<S, E> {
        &self.default_state
    }
}

impl<S, E> PseudoState<S, E> for HistoryPseudoState<S, E>
where
    S: PartialEq + Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn get_kind(&self) -> PseudoStateKind {
        self.kind
    }

    fn entry<'a>(
        &'a self,
        _context: &'a dyn StateContext<S, E>,
    ) -> BoxFuture<'a, Option<Arc<dyn StateMachineState<S, E>>>> {
        Box::pin(async move {
            let recorded = self.get_state();
            let default = self.default_state.get_state();

            match recorded {
                None => default.or_else(|| self.containing_state.get_state()),
                Some(state) => {
                    let is_end = state
                        .pseudo_state()
                        .map(|pseudo_state| pseudo_state.get_kind() == PseudoStateKind::End)
                        .unwrap_or(false);

                    if default.is_some() && is_end {
                        default
                    } else {
                        Some(state)
                    }
                }
            }
        })
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
