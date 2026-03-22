use std::sync::Arc;

use futures::FutureExt;
use next_web_core::async_trait;
use tracing::warn;

use crate::state_machine::{
    security::security_rule::SecurityRule,
    state::{action_listener::ActionListener, StateMachineState},
    state_context::StateContext,
    transition::{transition_kind::TransitionKind, StateMachineTransition},
    trigger::Trigger,
    BoxedStateAction, BoxedStateGuard,
};

/// Base implementation of a transition in a state machine.
///
/// This abstract class provides common functionality for state transitions,
/// including source/target states, guard conditions, actions, and triggers.
///
/// # Type Parameters
///
/// * `S` - The type of state
/// * `E` - The type of event
pub struct BaseTransition<S, E> {
    /// The source state of this transition.
    source: Option<Arc<dyn StateMachineState<S, E>>>,

    /// The target state of this transition.
    target: Arc<dyn StateMachineState<S, E>>,

    /// The kind of this transition (e.g., internal, external, local).
    kind: TransitionKind,

    /// The guard condition that must be satisfied for the transition to occur.
    guard: Option<BoxedStateGuard<S, E>>,

    /// The trigger that activates this transition.
    trigger: Option<Arc<dyn Trigger<S, E>>>,

    /// Security rules associated with this transition.
    security_rule: Option<SecurityRule>,

    /// The name of this transition.
    name: String,

    /// Actions to execute when this transition occurs.
    actions: Vec<BoxedStateAction<S, E>>,

    /// Listener for action execution events.
    action_listener: Option<Arc<dyn ActionListener<S, E>>>,
}

impl<S, E> BaseTransition<S, E> {
    /// Creates a new abstract transition.
    ///
    /// # Arguments
    ///
    /// * `source` - The source state
    /// * `target` - The target state
    /// * `actions` - Actions to execute during the transition
    /// * `event` - The event associated with this transition
    /// * `kind` - The kind of transition
    /// * `guard` - The guard condition function
    /// * `trigger` - The trigger for this transition
    pub fn new(
        source: Arc<dyn StateMachineState<S, E>>,
        target: Arc<dyn StateMachineState<S, E>>,
        actions: Vec<BoxedStateAction<S, E>>,
        _event: E,
        kind: TransitionKind,
        guard: Option<BoxedStateGuard<S, E>>,
        trigger: Option<Arc<dyn Trigger<S, E>>>,
    ) -> Self {
        Self {
            source: Some(source),
            target,
            kind,
            guard,
            trigger,
            security_rule: None,
            name: String::new(),
            actions,
            action_listener: None,
        }
    }

    /// Creates a new abstract transition with security rule and name.
    ///
    /// # Arguments
    ///
    /// * `source` - The source state
    /// * `target` - The target state
    /// * `actions` - Actions to execute during the transition
    /// * `event` - The event associated with this transition
    /// * `kind` - The kind of transition
    /// * `guard` - The guard condition function
    /// * `trigger` - The trigger for this transition
    /// * `security_rule` - Security rules for this transition
    /// * `name` - The name of this transition
    pub fn with_security_and_name(
        source: Option<Arc<dyn StateMachineState<S, E>>>,
        target: Arc<dyn StateMachineState<S, E>>,
        actions: Vec<BoxedStateAction<S, E>>,
        _event: Option<E>,
        kind: TransitionKind,
        guard: Option<BoxedStateGuard<S, E>>,
        trigger: Option<Arc<dyn Trigger<S, E>>>,
        security_rule: Option<SecurityRule>,
        name: Option<String>,
    ) -> Self {
        Self {
            source,
            target,
            kind,
            guard,
            trigger,
            security_rule,
            name: name.unwrap_or_default(),
            actions,
            action_listener: None,
        }
    }
}

#[async_trait]
impl<S, E> StateMachineTransition<S, E> for BaseTransition<S, E>
where
    S: Send + Sync,
    S: 'static,
    E: Send + Sync,
    E: 'static,
{
    /// Transit this transition with a give state context.
    ///
    /// # Arguments
    /// * `context` - the state context
    ///
    /// # Returns
    /// A future that resolves to `true` if transition happened, `false` otherwise
    async fn transit(&self, context: &dyn StateContext<S, E>) -> bool {
        let guard = match self.guard.as_ref() {
            Some(guard) => guard,
            None => return true,
        };

        let future = std::panic::AssertUnwindSafe(guard(context));

        match future.catch_unwind().await {
            Ok(value) => value,
            Err(_) => {
                warn!("Deny guard due to panic - guard should not panic");
                false
            }
        }
    }

    /// Execute transition actions.
    ///
    /// # Arguments
    /// * `context` - the state context
    ///
    /// # Returns
    /// A future that completes when all transition actions have been executed
    async fn execute_transition_actions(&self, context: &dyn StateContext<S, E>) {
        if self.actions.is_empty() {
            return;
        }

        for action in self.actions.iter() {
            let start_time = std::time::Instant::now();

            // Execute the action
            action(context).await;

            // Notify the action listener if present
            if let Some(listener) = &self.action_listener {
                let duration = start_time.elapsed();
                if let Some(state_machine) = context.state_machine() {
                    listener.on_execute(state_machine, action, duration).await;
                }
            }
        }
    }

    /// Gets the source state of this transition.
    ///
    /// # Returns
    /// The source state
    fn source(&self) -> &dyn StateMachineState<S, E> {
        self.source.as_ref().map(AsRef::as_ref).unwrap()
    }

    /// Gets the target state of this transition.
    ///
    /// # Returns
    /// The target state
    fn target(&self) -> &dyn StateMachineState<S, E> {
        self.target.as_ref()
    }

    /// Gets the guard of this transition.
    ///
    /// # Returns
    /// The guard function that determines if the transition can be taken
    fn guard(&self) -> Option<&BoxedStateGuard<S, E>> {
        self.guard.as_ref()
    }

    /// Gets the transition actions.
    ///
    /// # Returns
    /// A collection of action functions to execute during the transition
    fn actions(&self) -> &[BoxedStateAction<S, E>] {
        &self.actions
    }

    /// Gets the transition trigger.
    ///
    /// # Returns
    /// The trigger that initiates this transition
    fn trigger(&self) -> Option<&dyn Trigger<S, E>> {
        self.trigger.as_deref()
    }

    /// Gets the transition kind.
    ///
    /// # Returns
    /// The kind of transition
    fn kind(&self) -> TransitionKind {
        self.kind
    }

    /// Gets the security rule.
    ///
    /// # Returns
    /// The security rule for this transition, if any
    fn security_rule(&self) -> Option<&SecurityRule> {
        self.security_rule.as_ref()
    }

    /// Gets the name.
    ///
    /// # Returns
    /// The name of this transition
    fn name(&self) -> &str {
        &self.name
    }

    /// Adds the action listener.
    ///
    /// # Arguments
    /// * `listener` - the listener
    fn add_action_listener(&mut self, listener: Arc<dyn ActionListener<S, E>>) {
        self.action_listener = Some(listener);
    }

    /// Removes the action listener.
    ///
    /// # Arguments
    /// * `listener` - the listener
    fn remove_action_listener(&mut self, listener: &dyn ActionListener<S, E>) {
        if let Some(current_listener) = &self.action_listener {
            // Compare listener references
            if std::ptr::eq(
                current_listener.as_ref() as *const _ as *const (),
                listener as *const _ as *const (),
            ) {
                self.action_listener = None;
            }
        }
    }
}
