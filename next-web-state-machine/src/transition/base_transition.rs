//! Base implementation of a transition.

use std::sync::Arc;

use futures::FutureExt;
use next_web_core::async_trait;
use next_web_core::error::BoxError;
use tracing::warn;

use crate::BoxedStateAction;
use crate::BoxedStateGuard;
use crate::action::composite_action_listener::CompositeActionListener;
use crate::security::security_rule::SecurityRule;
use crate::state::{StateMachineState, action_listener::ActionListener};
use crate::state_context::StateContext;
use crate::transition::{StateMachineTransition, transition_kind::TransitionKind};
use crate::trigger::Trigger;

/// Base implementation of a transition in a state machine.
///
/// The Java class hierarchy splits this into `AbstractTransition`,
/// `AbstractExternalTransition`, `AbstractInternalTransition` and
/// `AbstractLocalTransition`; the Rust port keeps the kind in a field, because
/// the only difference between the subclasses is how the machine treats the
/// source and target state, which the machine decides on [`TransitionKind`].
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct BaseTransition<S, E> {
    source: Option<Arc<dyn StateMachineState<S, E>>>,
    target: Arc<dyn StateMachineState<S, E>>,
    kind: TransitionKind,
    guard: Option<BoxedStateGuard<S, E>>,
    trigger: Option<Arc<dyn Trigger<S, E>>>,
    security_rule: Option<SecurityRule>,
    name: String,
    actions: Vec<BoxedStateAction<S, E>>,
    action_listener: CompositeActionListener<S, E>,
}

impl<S, E> BaseTransition<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    /// Creates a new transition.
    ///
    /// # Arguments
    /// * `source` - the source state, `None` for an initial transition
    /// * `target` - the target state
    /// * `actions` - the actions to execute
    /// * `kind` - the kind of the transition
    /// * `guard` - the guard condition
    /// * `trigger` - the trigger of the transition
    pub fn new(
        source: Option<Arc<dyn StateMachineState<S, E>>>,
        target: Arc<dyn StateMachineState<S, E>>,
        actions: Vec<BoxedStateAction<S, E>>,
        kind: TransitionKind,
        guard: Option<BoxedStateGuard<S, E>>,
        trigger: Option<Arc<dyn Trigger<S, E>>>,
    ) -> Self {
        Self {
            source,
            target,
            kind,
            guard,
            trigger,
            security_rule: None,
            name: String::new(),
            actions,
            action_listener: CompositeActionListener::new(),
        }
    }

    /// Sets the security rule of this transition.
    ///
    /// # Arguments
    /// * `security_rule` - the security rule
    pub fn with_security_rule(mut self, security_rule: Option<SecurityRule>) -> Self {
        self.security_rule = security_rule;
        self
    }

    /// Sets the name of this transition.
    ///
    /// # Arguments
    /// * `name` - the transition name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Gets the guard as a shared handle, for the machine and the executor.
    pub fn guard_arc(&self) -> Option<BoxedStateGuard<S, E>> {
        self.guard.clone()
    }
}

#[async_trait]
impl<S, E> StateMachineTransition<S, E> for BaseTransition<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    async fn transit(&self, context: &dyn StateContext<S, E>) -> bool {
        let Some(guard) = self.guard.as_ref() else {
            return true;
        };

        if let Ok(value) = std::panic::AssertUnwindSafe(guard(context))
            .catch_unwind()
            .await { value } else {
            warn!("denying the transition because the guard panicked");
            false
        }
    }

    async fn execute_transition_actions(
        &self,
        context: &dyn StateContext<S, E>,
    ) -> Result<(), BoxError> {
        if self.actions.is_empty() {
            return Ok(());
        }

        for action in &self.actions {
            let started = std::time::Instant::now();
            action(context).await?;
            let elapsed = started.elapsed();
            if let Some(state_machine) = context.state_machine() {
                self.action_listener
                    .notify(state_machine, action, elapsed)
                    .await;
                state_machine.notify_action_executed(action, elapsed);
            }
        }

        Ok(())
    }

    fn source(&self) -> Option<Arc<dyn StateMachineState<S, E>>> {
        self.source.clone()
    }

    fn target(&self) -> Arc<dyn StateMachineState<S, E>> {
        self.target.clone()
    }

    fn guard(&self) -> Option<&BoxedStateGuard<S, E>> {
        self.guard.as_ref()
    }

    fn actions(&self) -> &[BoxedStateAction<S, E>] {
        &self.actions
    }

    fn trigger(&self) -> Option<&Arc<dyn Trigger<S, E>>> {
        self.trigger.as_ref()
    }

    fn kind(&self) -> TransitionKind {
        self.kind
    }

    fn security_rule(&self) -> Option<&SecurityRule> {
        self.security_rule.as_ref()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn add_action_listener(&self, listener: Arc<dyn ActionListener<S, E>>) {
        self.action_listener.register(listener);
    }

    fn remove_action_listener(&self, listener: Arc<dyn ActionListener<S, E>>) {
        self.action_listener.unregister(listener);
    }
}
