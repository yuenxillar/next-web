use std::sync::Arc;

use next_web_core::async_trait;
use next_web_core::error::BoxError;

use crate::{
    BoxedStateAction, BoxedStateGuard,
    security::security_rule::SecurityRule,
    state::{StateMachineState, action_listener::ActionListener},
    state_context::StateContext,
    transition::{
        StateMachineTransition, base_transition::BaseTransition, transition_kind::TransitionKind,
    },
    trigger::Trigger,
};

/// A transition used during state machine initialization.
///
/// It has no source state and its `transit` is a no-op: the machine applies the
/// initial transition itself when it starts.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct InitialTransition<S, E> {
    base_transition: BaseTransition<S, E>,
}

impl<S, E> InitialTransition<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    /// Creates a new initial transition with the given target state.
    ///
    /// # Arguments
    /// * `target` - the initial state of the state machine
    pub fn new(target: Arc<dyn StateMachineState<S, E>>) -> Self {
        Self::with_actions(target, Vec::new())
    }

    /// Creates a new initial transition with a single action.
    ///
    /// # Arguments
    /// * `target` - the initial state of the state machine
    /// * `action` - the action executed by the transition
    pub fn with_action(
        target: Arc<dyn StateMachineState<S, E>>,
        action: BoxedStateAction<S, E>,
    ) -> Self {
        Self::with_actions(target, vec![action])
    }

    /// Creates a new initial transition with several actions.
    ///
    /// # Arguments
    /// * `target` - the initial state of the state machine
    /// * `actions` - the actions executed by the transition
    pub fn with_actions(
        target: Arc<dyn StateMachineState<S, E>>,
        actions: Vec<BoxedStateAction<S, E>>,
    ) -> Self {
        Self {
            base_transition: BaseTransition::new(
                None,
                target,
                actions,
                TransitionKind::Initial,
                None,
                None,
            ),
        }
    }

    /// Gets the wrapped base transition.
    pub fn base_transition(&self) -> &BaseTransition<S, E> {
        &self.base_transition
    }
}

#[async_trait]
impl<S, E> StateMachineTransition<S, E> for InitialTransition<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    async fn transit(&self, _context: &dyn StateContext<S, E>) -> bool {
        // The machine applies the initial transition itself.
        false
    }

    async fn execute_transition_actions(
        &self,
        context: &dyn StateContext<S, E>,
    ) -> Result<(), BoxError> {
        self.base_transition
            .execute_transition_actions(context)
            .await
    }

    fn source(&self) -> Option<Arc<dyn StateMachineState<S, E>>> {
        None
    }

    fn target(&self) -> Arc<dyn StateMachineState<S, E>> {
        self.base_transition.target()
    }

    fn guard(&self) -> Option<&BoxedStateGuard<S, E>> {
        self.base_transition.guard()
    }

    fn actions(&self) -> &[BoxedStateAction<S, E>] {
        self.base_transition.actions()
    }

    fn trigger(&self) -> Option<&Arc<dyn Trigger<S, E>>> {
        self.base_transition.trigger()
    }

    fn kind(&self) -> TransitionKind {
        TransitionKind::Initial
    }

    fn security_rule(&self) -> Option<&SecurityRule> {
        self.base_transition.security_rule()
    }

    fn name(&self) -> &str {
        self.base_transition.name()
    }

    fn add_action_listener(&self, listener: Arc<dyn ActionListener<S, E>>) {
        self.base_transition.add_action_listener(listener);
    }

    fn remove_action_listener(&self, listener: Arc<dyn ActionListener<S, E>>) {
        self.base_transition.remove_action_listener(listener);
    }
}
