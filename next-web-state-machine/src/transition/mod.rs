//! Transitions move a state machine from one state to another.

/// The base of every transition.
pub mod base_transition;
/// The transition used when a machine enters a local state.
pub mod default_local_transition;
/// The transition which runs when a machine starts.
pub mod initial_transition;
/// The policy which resolves conflicting transitions.
pub mod transition_conflict_policy;
/// The kinds of a transition.
pub mod transition_kind;

use std::sync::Arc;

use next_web_core::{async_trait, error::BoxError};

use crate::{
    BoxedStateAction, BoxedStateGuard,
    security::security_rule::SecurityRule,
    state::{StateMachineState, action_listener::ActionListener},
    state_context::StateContext,
    transition::transition_kind::TransitionKind,
    trigger::Trigger,
};

/// `StateMachineTransition` is what a state machine associates with a change of
/// its state.
///
/// The Java `Transition` returns `null` from `getSource()` for an initial
/// transition, which the Rust port models with `Option` so nothing has to panic.
/// Sources and targets are handed out as shared handles, because a transition
/// outlives the state machine build and Rust has no garbage collector to keep
/// the referenced state alive.
///
/// # Type Parameters
/// * `S`: the type of state
/// * `E`: the type of event
#[async_trait]
pub trait StateMachineTransition<S, E>: Send + Sync {
    /// Transits this transition with the given state context, which evaluates
    /// the guard of the transition.
    ///
    /// # Arguments
    /// * `context` - the state context
    ///
    /// # Returns
    /// `true` if the transition happened, `false` otherwise
    async fn transit(&self, context: &dyn StateContext<S, E>) -> bool;

    /// Executes the transition actions.
    ///
    /// # Arguments
    /// * `context` - the state context
    async fn execute_transition_actions(
        &self,
        context: &dyn StateContext<S, E>,
    ) -> Result<(), BoxError>;

    /// Gets the source state of this transition.
    ///
    /// An initial transition has no source state.
    fn source(&self) -> Option<Arc<dyn StateMachineState<S, E>>>;

    /// Gets the target state of this transition.
    fn target(&self) -> Arc<dyn StateMachineState<S, E>>;

    /// Gets the guard of this transition.
    fn guard(&self) -> Option<&BoxedStateGuard<S, E>>;

    /// Gets the transition actions.
    fn actions(&self) -> &[BoxedStateAction<S, E>];

    /// Gets the transition trigger.
    fn trigger(&self) -> Option<&Arc<dyn Trigger<S, E>>>;

    /// Gets the transition kind.
    fn kind(&self) -> TransitionKind;

    /// Gets the security rule.
    fn security_rule(&self) -> Option<&SecurityRule>;

    /// Gets the name.
    fn name(&self) -> &str;

    /// Adds the action listener.
    ///
    /// # Arguments
    /// * `listener` - the listener
    fn add_action_listener(&self, listener: Arc<dyn ActionListener<S, E>>);

    /// Removes the action listener.
    ///
    /// # Arguments
    /// * `listener` - the listener
    fn remove_action_listener(&self, listener: Arc<dyn ActionListener<S, E>>);
}
