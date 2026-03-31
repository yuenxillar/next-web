use std::sync::Arc;

use next_web_core::async_trait;

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
/// This transition represents the initial transition when a state machine
/// starts. It transitions from no state (represented as `None`) to the
/// initial state specified as the target.
///
/// # Type Parameters
///
/// * `S` - The type of state
/// * `E` - The type of event
pub struct InitialTransition<S, E> {
    /// The base transition functionality
    base_transition: BaseTransition<S, E>,
}

impl<S, E> InitialTransition<S, E> {
    /// Creates a new initial transition with the specified target state.
    ///
    /// # Arguments
    ///
    /// * `target` - The initial state of the state machine
    pub fn new(target: Arc<dyn StateMachineState<S, E>>) -> Self {
        Self::from_base(BaseTransition::with_security_and_name(
            None, // No source state for initial transition
            target,
            Vec::new(), // No actions
            None,       // No triggering event
            TransitionKind::Initial,
            None, // No guard condition
            None, // No trigger
            None,
            None,
        ))
    }

    /// Creates a new initial transition with a single action.
    ///
    /// # Arguments
    ///
    /// * `target` - The initial state of the state machine
    /// * `action` - The action to execute during the initial transition
    pub fn with_action(
        target: Arc<dyn StateMachineState<S, E>>,
        action: BoxedStateAction<S, E>,
    ) -> Self {
        let actions = vec![action];

        Self::from_base(BaseTransition::with_security_and_name(
            None,
            target,
            actions,
            None,
            TransitionKind::Initial,
            None,
            None,
            None,
            None,
        ))
    }

    /// Creates a new initial transition with multiple actions.
    ///
    /// # Arguments
    ///
    /// * `target` - The initial state of the state machine
    /// * `actions` - The actions to execute during the initial transition
    pub fn with_actions(
        target: Arc<dyn StateMachineState<S, E>>,
        actions: Vec<BoxedStateAction<S, E>>,
    ) -> Self {
        Self::from_base(BaseTransition::with_security_and_name(
            None,
            target,
            actions,
            None,
            TransitionKind::Initial,
            None,
            None,
            None,
            None,
        ))
    }

    /// Creates an `InitialTransition` from an existing `BaseTransition`.
    ///
    /// # Arguments
    ///
    /// * `base` - The base transition to wrap
    fn from_base(base: BaseTransition<S, E>) -> Self {
        InitialTransition {
            base_transition: base,
        }
    }
}

#[async_trait]
impl<S, E> StateMachineTransition<S, E> for InitialTransition<S, E>
where
    S: Send + Sync,
    S: 'static,
    E: Send + Sync,
    E: 'static,
{
    /// Attempts to execute the initial transition.
    ///
    /// For initial transitions, this always returns `false` because
    /// the initial transition itself does not cause further state changes.
    ///
    /// # Arguments
    ///
    /// * `context` - The current state context
    ///
    /// # Returns
    ///
    /// `false` indicating no further transitions should occur
    #[allow(unused_variables)]
    async fn transit(&self, context: &dyn StateContext<S, E>) -> bool {
        // Initial transition itself doesn't cause further changes.
        // Returning false indicates that no additional transitions should occur
        // as a result of this initial transition.
        false
    }

    /// Gets the source state of this transition.
    ///
    /// For initial transitions, this always returns `None`.
    ///
    /// # Returns
    ///
    /// `None` as initial transitions have no source state
    fn source(&self) -> &dyn StateMachineState<S, E> {
        self.base_transition.source()
    }

    /// Gets the target state of this transition.
    ///
    /// # Returns
    ///
    /// The initial state of the state machine
    fn target(&self) -> &dyn StateMachineState<S, E> {
        self.base_transition.target()
    }

    /// Gets the kind of this transition.
    ///
    /// # Returns
    ///
    /// Always returns `TransitionKind::Initial`
    fn kind(&self) -> TransitionKind {
        self.base_transition.kind()
    }

    /// Gets the guard condition function.
    ///
    /// For initial transitions, this always returns `None`.
    ///
    /// # Returns
    ///
    /// `None` as initial transitions have no guard condition
    fn guard(&self) -> Option<&BoxedStateGuard<S, E>> {
        self.base_transition.guard()
    }

    /// Gets the trigger associated with this transition.
    ///
    /// For initial transitions, this always returns `None`.
    ///
    /// # Returns
    ///
    /// `None` as initial transitions have no trigger
    fn trigger(&self) -> Option<&dyn Trigger<S, E>> {
        self.base_transition.trigger()
    }

    /// Gets the security rule associated with this transition.
    ///
    /// # Returns
    ///
    /// The security rule, if any
    fn security_rule(&self) -> Option<&SecurityRule> {
        self.base_transition.security_rule()
    }

    /// Gets the name of this transition.
    ///
    /// # Returns
    ///
    /// The transition name, if any
    fn name(&self) -> &str {
        self.base_transition.name()
    }

    /// Gets the actions associated with this transition.
    ///
    /// # Returns
    ///
    /// A slice of action functions
    fn actions(&self) -> &[BoxedStateAction<S, E>] {
        self.base_transition.actions()
    }

    /// Adds an action listener to this transition.
    ///
    /// # Arguments
    ///
    /// * `listener` - The action listener to add
    fn add_action_listener(&mut self, listener: Arc<dyn ActionListener<S, E>>) {
        self.base_transition.add_action_listener(listener);
    }

    /// Removes an action listener from this transition.
    ///
    /// # Arguments
    ///
    /// * `listener` - The action listener to remove
    fn remove_action_listener(&mut self, listener: &dyn ActionListener<S, E>) {
        self.base_transition.remove_action_listener(listener);
    }

    /// Executes all transition actions in sequence.
    ///
    /// # Arguments
    ///
    /// * `context` - The state context for action execution
    async fn execute_transition_actions(&self, context: &dyn StateContext<S, E>) {
        self.base_transition
            .execute_transition_actions(context)
            .await;
    }
}
