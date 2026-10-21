//! The junction pseudo state.

use std::sync::Arc;

use futures::future::BoxFuture;
use next_web_core::error::BoxError;
use tracing::warn;

use crate::BoxedStateAction;
use crate::BoxedStateGuard;
use crate::state::StateMachineState;
use crate::state::pseudo_state::PseudoState;
use crate::state::pseudo_state_context::PseudoStateContext;
use crate::state::pseudo_state_kind::PseudoStateKind;
use crate::state::pseudo_state_listener::PseudoStateListener;
use crate::state_context::StateContext;

/// Data class wrapping a junction target state and its guard.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct JunctionStateData<S, E> {
    state: Arc<dyn StateMachineState<S, E>>,
    guard: Option<BoxedStateGuard<S, E>>,
    actions: Vec<BoxedStateAction<S, E>>,
}

impl<S, E> JunctionStateData<S, E> {
    /// Creates a new junction entry.
    ///
    /// # Arguments
    /// * `state` - the target state
    /// * `guard` - the guard which selects this entry
    /// * `actions` - the actions executed when this entry is selected
    pub fn new(
        state: Arc<dyn StateMachineState<S, E>>,
        guard: Option<BoxedStateGuard<S, E>>,
        actions: Vec<BoxedStateAction<S, E>>,
    ) -> Self {
        Self {
            state,
            guard,
            actions,
        }
    }

    /// Gets the target state.
    pub fn get_state(&self) -> &Arc<dyn StateMachineState<S, E>> {
        &self.state
    }

    /// Gets the guard of this entry.
    pub fn get_guard(&self) -> Option<&BoxedStateGuard<S, E>> {
        self.guard.as_ref()
    }

    /// Gets the actions of this entry.
    pub fn get_actions(&self) -> &[BoxedStateAction<S, E>] {
        &self.actions
    }
}

/// Junction implementation of a [`PseudoState`].
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct JunctionPseudoState<S, E> {
    junctions: Vec<JunctionStateData<S, E>>,
}

impl<S, E> JunctionPseudoState<S, E> {
    /// Creates a new junction pseudo state.
    ///
    /// # Arguments
    /// * `junctions` - the selectable entries
    pub fn new(junctions: Vec<JunctionStateData<S, E>>) -> Self {
        Self { junctions }
    }

    /// Gets the selectable entries.
    pub fn get_junctions(&self) -> &[JunctionStateData<S, E>] {
        &self.junctions
    }
}

impl<S, E> PseudoState<S, E> for JunctionPseudoState<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn get_kind(&self) -> PseudoStateKind {
        PseudoStateKind::Junction
    }

    fn entry<'a>(
        &'a self,
        context: &'a dyn StateContext<S, E>,
    ) -> BoxFuture<'a, Option<Arc<dyn StateMachineState<S, E>>>> {
        Box::pin(async move {
            let mut selected = None;
            for junction in &self.junctions {
                selected = Some(junction);
                if let Some(guard) = junction.guard.as_ref()
                    && guard(context).await {
                        break;
                    }
            }

            let junction = selected?;
            for action in &junction.actions {
                if let Err(error) = action(context).await {
                    warn!(%error, "junction action returned an error");
                }
            }

            Some(junction.state.clone())
        })
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
