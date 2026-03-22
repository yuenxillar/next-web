pub mod base_transition;
pub mod default_local_transition;
pub mod initial_transition;
pub mod transition_conflict_policy;
pub mod transition_kind;

use std::sync::Arc;

use next_web_core::async_trait;

use crate::state_machine::{
    security::security_rule::SecurityRule,
    state::{action_listener::ActionListener, StateMachineState},
    state_context::StateContext,
    transition::transition_kind::TransitionKind,
    trigger::Trigger,
    BoxedStateAction, BoxedStateGuard,
};

/// `Transition` is something what a state machine associates with a state
/// changes.
///
/// This trait defines the core functionality of a transition between states
/// in a state machine, including execution, guard conditions, actions,
/// triggers, and lifecycle management.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
#[async_trait]
pub trait StateMachineTransition<S, E>
where
    S: Send,
    E: Send,
    Self: Send + Sync,
{
    /// Transit this transition with a give state context.
    ///
    /// # Arguments
    /// * `context` - the state context
    ///
    /// # Returns
    /// A future that resolves to `true` if transition happened, `false` otherwise
    async fn transit(&self, context: &dyn StateContext<S, E>) -> bool;

    /// Execute transition actions.
    ///
    /// # Arguments
    /// * `context` - the state context
    ///
    /// # Returns
    /// A future that completes when all transition actions have been executed
    async fn execute_transition_actions(&self, context: &dyn StateContext<S, E>);

    /// Gets the source state of this transition.
    ///
    /// # Returns
    /// The source state
    fn source(&self) -> &dyn StateMachineState<S, E>;

    /// Gets the target state of this transition.
    ///
    /// # Returns
    /// The target state
    fn target(&self) -> &dyn StateMachineState<S, E>;

    /// Gets the guard of this transition.
    ///
    /// # Returns
    /// The guard function that determines if the transition can be taken
    fn guard(&self) -> Option<&BoxedStateGuard<S, E>>;

    /// Gets the transition actions.
    ///
    /// # Returns
    /// A collection of action functions to execute during the transition
    fn actions(&self) -> &[BoxedStateAction<S, E>];

    /// Gets the transition trigger.
    ///
    /// # Returns
    /// The trigger that initiates this transition
    fn trigger(&self) -> Option<&dyn Trigger<S, E>>;

    /// Gets the transition kind.
    ///
    /// # Returns
    /// The kind of transition
    fn kind(&self) -> TransitionKind;

    /// Gets the security rule.
    ///
    /// # Returns
    /// The security rule for this transition, if any
    fn security_rule(&self) -> Option<&SecurityRule>;

    /// Gets the name.
    ///
    /// # Returns
    /// The name of this transition
    fn name(&self) -> &str;

    /// Adds the action listener.
    ///
    /// # Arguments
    /// * `listener` - the listener
    fn add_action_listener(&mut self, listener: Arc<dyn ActionListener<S, E>>);

    /// Removes the action listener.
    ///
    /// # Arguments
    /// * `listener` - the listener
    fn remove_action_listener(&mut self, listener: &dyn ActionListener<S, E>);
}
