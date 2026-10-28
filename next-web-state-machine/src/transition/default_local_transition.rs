//! A local transition, usable when a transition should stay inside the source
//! state when the target is a substate of it.

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

/// Represents a default local transition implementation.
///
/// The kind is what makes the machine keep the source state active, so this is
/// a thin wrapper over [`BaseTransition`].
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct DefaultLocalTransition<S, E>(BaseTransition<S, E>);

impl<S, E> DefaultLocalTransition<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    /// Instantiates a new default local transition.
    ///
    /// # Arguments
    /// * `source` - the source state
    /// * `target` - the target state
    /// * `actions` - the actions to execute during the transition
    /// * `guard` - the guard of the transition
    /// * `trigger` - the trigger of the transition
    pub fn new(
        source: Arc<dyn StateMachineState<S, E>>,
        target: Arc<dyn StateMachineState<S, E>>,
        actions: Vec<BoxedStateAction<S, E>>,
        guard: Option<BoxedStateGuard<S, E>>,
        trigger: Option<Arc<dyn Trigger<S, E>>>,
    ) -> Self {
        Self(BaseTransition::new(
            Some(source),
            target,
            actions,
            TransitionKind::Local,
            guard,
            trigger,
        ))
    }

    /// Instantiates a new default local transition with a security rule.
    ///
    /// # Arguments
    /// * `source` - the source state
    /// * `target` - the target state
    /// * `actions` - the actions to execute during the transition
    /// * `guard` - the guard of the transition
    /// * `trigger` - the trigger of the transition
    /// * `security_rule` - the security rule applied to the transition
    pub fn with_security_rule(
        source: Option<Arc<dyn StateMachineState<S, E>>>,
        target: Arc<dyn StateMachineState<S, E>>,
        actions: Vec<BoxedStateAction<S, E>>,
        guard: Option<BoxedStateGuard<S, E>>,
        trigger: Option<Arc<dyn Trigger<S, E>>>,
        security_rule: SecurityRule,
    ) -> Self {
        Self(
            BaseTransition::new(
                source,
                target,
                actions,
                TransitionKind::Local,
                guard,
                trigger,
            )
            .with_security_rule(Some(security_rule)),
        )
    }

    /// Instantiates a new default local transition with a security rule and a name.
    ///
    /// # Arguments
    /// * `source` - the source state
    /// * `target` - the target state
    /// * `actions` - the actions to execute during the transition
    /// * `guard` - the guard of the transition
    /// * `trigger` - the trigger of the transition
    /// * `security_rule` - the security rule applied to the transition
    /// * `name` - the name of the transition
    pub fn with_security_rule_and_name(
        source: Option<Arc<dyn StateMachineState<S, E>>>,
        target: Arc<dyn StateMachineState<S, E>>,
        actions: Vec<BoxedStateAction<S, E>>,
        guard: Option<BoxedStateGuard<S, E>>,
        trigger: Option<Arc<dyn Trigger<S, E>>>,
        security_rule: SecurityRule,
        name: String,
    ) -> Self {
        Self(
            BaseTransition::new(
                source,
                target,
                actions,
                TransitionKind::Local,
                guard,
                trigger,
            )
            .with_security_rule(Some(security_rule))
            .with_name(name),
        )
    }
}

#[async_trait]
impl<S, E> StateMachineTransition<S, E> for DefaultLocalTransition<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    async fn transit(&self, context: &dyn StateContext<S, E>) -> bool {
        self.0.transit(context).await
    }

    async fn execute_transition_actions(
        &self,
        context: &dyn StateContext<S, E>,
    ) -> Result<(), BoxError> {
        self.0.execute_transition_actions(context).await
    }

    fn source(&self) -> Option<Arc<dyn StateMachineState<S, E>>> {
        self.0.source()
    }

    fn target(&self) -> Arc<dyn StateMachineState<S, E>> {
        self.0.target()
    }

    fn guard(&self) -> Option<&BoxedStateGuard<S, E>> {
        self.0.guard()
    }

    fn actions(&self) -> &[BoxedStateAction<S, E>] {
        self.0.actions()
    }

    fn trigger(&self) -> Option<&Arc<dyn Trigger<S, E>>> {
        self.0.trigger()
    }

    fn kind(&self) -> TransitionKind {
        TransitionKind::Local
    }

    fn security_rule(&self) -> Option<&SecurityRule> {
        self.0.security_rule()
    }

    fn name(&self) -> &str {
        self.0.name()
    }

    fn add_action_listener(&self, listener: Arc<dyn ActionListener<S, E>>) {
        self.0.add_action_listener(listener);
    }

    fn remove_action_listener(&self, listener: Arc<dyn ActionListener<S, E>>) {
        self.0.remove_action_listener(listener);
    }
}
