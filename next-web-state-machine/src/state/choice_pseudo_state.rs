//! The choice pseudo state.

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

/// Data class wrapping a choice target state and its guard.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct ChoiceStateData<S, E> {
    state: Arc<dyn StateMachineState<S, E>>,
    guard: Option<BoxedStateGuard<S, E>>,
    actions: Vec<BoxedStateAction<S, E>>,
}

impl<S, E> ChoiceStateData<S, E> {
    /// Creates a new choice entry.
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

/// Choice implementation of a [`PseudoState`].
///
/// The first entry whose guard evaluates to `true` wins; if no guard matches,
/// the last entry is taken, which is what the Java implementation does.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct ChoicePseudoState<S, E> {
    choices: Vec<ChoiceStateData<S, E>>,
}

impl<S, E> ChoicePseudoState<S, E> {
    /// Creates a new choice pseudo state.
    ///
    /// # Arguments
    /// * `choices` - the selectable entries
    pub fn new(choices: Vec<ChoiceStateData<S, E>>) -> Self {
        Self { choices }
    }

    /// Gets the selectable entries.
    pub fn get_choices(&self) -> &[ChoiceStateData<S, E>] {
        &self.choices
    }
}

impl<S, E> PseudoState<S, E> for ChoicePseudoState<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn get_kind(&self) -> PseudoStateKind {
        PseudoStateKind::Choice
    }

    fn entry<'a>(
        &'a self,
        context: &'a dyn StateContext<S, E>,
    ) -> BoxFuture<'a, Option<Arc<dyn StateMachineState<S, E>>>> {
        Box::pin(async move {
            let mut selected = None;
            for choice in &self.choices {
                selected = Some(choice);
                if let Some(guard) = choice.guard.as_ref()
                    && guard(context).await {
                        break;
                    }
            }

            let choice = selected?;
            for action in &choice.actions {
                if let Err(error) = action(context).await {
                    warn!(%error, "choice action returned an error");
                }
            }

            Some(choice.state.clone())
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
