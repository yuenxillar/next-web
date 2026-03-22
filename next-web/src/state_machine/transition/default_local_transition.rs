use std::sync::Arc;

use crate::state_machine::{
    security::security_rule::SecurityRule,
    state::StateMachineState,
    transition::{base_transition::BaseTransition, transition_kind::TransitionKind},
    trigger::Trigger,
    BoxedStateAction, BoxedStateGuard,
};

/// Represents a default local transition implementation.
pub struct DefaultLocalTransition<S, E>(BaseTransition<S, E>);

impl<S, E> DefaultLocalTransition<S, E> {
    /// Instantiates a new default local transition.
    ///
    /// # Arguments
    ///
    /// * `source` - The source state.
    /// * `target` - The target state.
    /// * `actions` - The collection of actions to execute during the transition.
    /// * `event` - The event triggering the transition.
    /// * `guard` - The function acting as a guard condition for the transition.
    /// * `trigger` - The trigger object associated with the transition.
    pub fn new(
        source: Arc<dyn StateMachineState<S, E>>,
        target: Arc<dyn StateMachineState<S, E>>,
        actions: Vec<BoxedStateAction<S, E>>,
        event: E,
        guard: Option<BoxedStateGuard<S, E>>,
        trigger: Option<Arc<dyn Trigger<S, E>>>,
    ) -> Self {
        Self(BaseTransition::new(
            source,
            target,
            actions,
            event,
            TransitionKind::Local,
            guard,
            trigger,
        ))
    }

    /// Instantiates a new default local transition with a security rule.
    ///
    /// # Arguments
    ///
    /// * `source` - The source state.
    /// * `target` - The target state.
    /// * `actions` - The collection of actions to execute during the transition.
    /// * `event` - The event triggering the transition.
    /// * `guard` - The function acting as a guard condition for the transition.
    /// * `trigger` - The trigger object associated with the transition.
    /// * `security_rule` - The security rule applied to the transition.
    pub fn with_security_rule(
        source: Option<Arc<dyn StateMachineState<S, E>>>,
        target: Arc<dyn StateMachineState<S, E>>,
        actions: Vec<BoxedStateAction<S, E>>,
        event: Option<E>,
        guard: Option<BoxedStateGuard<S, E>>,
        trigger: Option<Arc<dyn Trigger<S, E>>>,
        security_rule: SecurityRule,
    ) -> Self {
        Self(BaseTransition::with_security_and_name(
            source,
            target,
            actions,
            event,
            TransitionKind::Local,
            guard,
            trigger,
            Some(security_rule),
            None,
        ))
    }

    /// Instantiates a new default local transition with a security rule and a name.
    ///
    /// # Arguments
    ///
    /// * `source` - The source state.
    /// * `target` - The target state.
    /// * `actions` - The collection of actions to execute during the transition.
    /// * `event` - The event triggering the transition.
    /// * `guard` - The function acting as a guard condition for the transition.
    /// * `trigger` - The trigger object associated with the transition.
    /// * `security_rule` - The security rule applied to the transition.
    /// * `name` - The optional name for the transition.
    pub fn with_security_rule_and_name(
        source: Option<Arc<dyn StateMachineState<S, E>>>,
        target: Arc<dyn StateMachineState<S, E>>,
        actions: Vec<BoxedStateAction<S, E>>,
        event: Option<E>,
        guard: Option<BoxedStateGuard<S, E>>,
        trigger: Option<Arc<dyn Trigger<S, E>>>,
        security_rule: SecurityRule,
        name: String,
    ) -> Self {
        Self(BaseTransition::with_security_and_name(
            source,
            target,
            actions,
            event,
            TransitionKind::Local,
            guard,
            trigger,
            Some(security_rule),
            Some(name),
        ))
    }
}
