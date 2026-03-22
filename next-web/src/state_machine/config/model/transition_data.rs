use std::sync::Arc;

use futures::future::BoxFuture;

use crate::state_machine::{
    security::security_rule::SecurityRule, state_context::StateContext,
    transition::transition_kind::TransitionKind,
};

pub type Action<S, E> = Arc<dyn Fn(&dyn StateContext<S, E>) -> BoxFuture<'static, ()>>;
pub type Guard<S, E> = Arc<dyn Fn(&dyn StateContext<S, E>) -> BoxFuture<'static, bool>>;

/// A simple data object keeping transition related configs in a same place.
///
/// This struct encapsulates all configuration data for a state machine transition,
/// including source/target states, triggers (event, timer, count), actions,
/// guard conditions, security rules, and metadata.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
#[derive(Clone)]
pub struct TransitionData<S, E> {
    /// Source state of the transition
    source: S,
    /// Target state of the transition
    target: S,
    /// State for internal/local transitions
    pub(crate) state: Option<S>,
    /// Event that triggers the transition
    event: Option<E>,
    /// Timer period for timed transitions
    period: Option<u64>,
    /// Event count for count-based transitions
    count: Option<u32>,
    /// Actions to execute during the transition
    actions: Vec<Action<S, E>>,
    /// Guard condition that must be satisfied
    guard: Option<Guard<S, E>>,
    /// Type of transition
    kind: TransitionKind,
    /// Security rule for the transition
    security_rule: Option<SecurityRule>,
    /// Name of the transition
    name: String,
}
impl<S, E> TransitionData<S, E>
where
    S: Clone + Eq,
    E: Clone + Eq,
{
    /// Creates a new transition data with source, target, and event.
    ///
    /// # Arguments
    /// * `source` - Source state
    /// * `target` - Target state
    /// * `event` - Triggering event
    ///
    /// # Returns
    /// A new `TransitionData` instance
    pub fn new(source: S, target: S, event: E) -> Self {
        Self {
            source,
            target,
            state: None,
            event: Some(event),
            period: None,
            count: None,
            actions: Vec::new(),
            guard: None,
            kind: TransitionKind::External,
            security_rule: None,
            name: String::new(),
        }
    }

    /// Creates a new transition data with event-based trigger.
    ///
    /// # Arguments
    /// * `source` - Source state
    /// * `target` - Target state
    /// * `event` - Triggering event
    /// * `actions` - Transition actions
    /// * `guard` - Guard condition
    /// * `kind` - Transition kind
    ///
    /// # Returns
    /// A new `TransitionData` instance
    pub fn with_event_trigger(
        source: S,
        target: S,
        event: E,
        actions: Vec<Action<S, E>>,
        guard: Option<Guard<S, E>>,
        kind: TransitionKind,
    ) -> Self {
        Self {
            source,
            target,
            state: None,
            event: Some(event),
            period: None,
            count: None,
            actions,
            guard,
            kind,
            security_rule: None,
            name: String::new(),
        }
    }

    /// Creates a new transition data with event-based trigger and name.
    ///
    /// # Arguments
    /// * `source` - Source state
    /// * `target` - Target state
    /// * `event` - Triggering event
    /// * `actions` - Transition actions
    /// * `guard` - Guard condition
    /// * `kind` - Transition kind
    /// * `name` - Transition name
    ///
    /// # Returns
    /// A new `TransitionData` instance
    pub fn with_event_trigger_named(
        source: S,
        target: S,
        event: E,
        actions: Vec<Action<S, E>>,
        guard: Option<Guard<S, E>>,
        kind: TransitionKind,
        name: impl Into<String>,
    ) -> Self {
        Self {
            source,
            target,
            state: None,
            event: Some(event),
            period: None,
            count: None,
            actions,
            guard,
            kind,
            security_rule: None,
            name: name.into(),
        }
    }

    /// Creates a new transition data with timer-based trigger.
    ///
    /// # Arguments
    /// * `source` - Source state
    /// * `target` - Target state
    /// * `period` - Timer period
    /// * `count` - Event count
    /// * `actions` - Transition actions
    /// * `guard` - Guard condition
    /// * `kind` - Transition kind
    ///
    /// # Returns
    /// A new `TransitionData` instance
    pub fn with_timer_trigger(
        source: S,
        target: S,
        period: Option<u64>,
        count: Option<u32>,
        actions: Vec<Action<S, E>>,
        guard: Option<Guard<S, E>>,
        kind: TransitionKind,
    ) -> Self {
        Self {
            source,
            target,
            state: None,
            event: None,
            period,
            count,
            actions,
            guard,
            kind,
            security_rule: None,
            name: String::new(),
        }
    }

    /// Creates a new transition data with timer-based trigger and name.
    ///
    /// # Arguments
    /// * `source` - Source state
    /// * `target` - Target state
    /// * `period` - Timer period
    /// * `count` - Event count
    /// * `actions` - Transition actions
    /// * `guard` - Guard condition
    /// * `kind` - Transition kind
    /// * `name` - Transition name
    ///
    /// # Returns
    /// A new `TransitionData` instance
    pub fn with_timer_trigger_named(
        source: S,
        target: S,
        period: Option<u64>,
        count: Option<u32>,
        actions: Vec<Action<S, E>>,
        guard: Option<Guard<S, E>>,
        kind: TransitionKind,
        name: impl Into<String>,
    ) -> Self {
        Self {
            source,
            target,
            state: None,
            event: None,
            period,
            count,
            actions,
            guard,
            kind,
            security_rule: None,
            name: name.into(),
        }
    }

    /// Creates a new transition data with all parameters.
    ///
    /// # Arguments
    /// * `source` - Source state
    /// * `target` - Target state
    /// * `state` - State for internal/local transitions
    /// * `event` - Triggering event
    /// * `period` - Timer period
    /// * `count` - Event count
    /// * `actions` - Transition actions
    /// * `guard` - Guard condition
    /// * `kind` - Transition kind
    /// * `security_rule` - Security rule
    /// * `name` - Transition name
    ///
    /// # Returns
    /// A new `TransitionData` instance
    pub fn with_all(
        source: S,
        target: S,
        state: Option<S>,
        event: Option<E>,
        period: Option<u64>,
        count: Option<u32>,
        actions: Vec<Action<S, E>>,
        guard: Option<Guard<S, E>>,
        kind: TransitionKind,
        security_rule: Option<SecurityRule>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            source,
            target,
            state,
            event,
            period,
            count,
            actions,
            guard,
            kind,
            security_rule,
            name: name.into(),
        }
    }

    /// Gets the source state.
    ///
    /// # Returns
    /// The source state, if any
    pub fn source(&self) -> &S {
        &self.source
    }

    /// Gets the target state.
    ///
    /// # Returns
    /// The target state, if any
    pub fn target(&self) -> &S {
        &self.target
    }

    /// Gets the state for internal/local transitions.
    ///
    /// # Returns
    /// The state, if any
    pub fn state(&self) -> Option<&S> {
        self.state.as_ref()
    }

    /// Gets the event that triggers the transition.
    ///
    /// # Returns
    /// The event, if any
    pub fn event(&self) -> Option<&E> {
        self.event.as_ref()
    }

    /// Gets the timer period for timed transitions.
    ///
    /// # Returns
    /// The period, if any
    pub fn period(&self) -> Option<u64> {
        self.period
    }

    /// Gets the event count for count-based transitions.
    ///
    /// # Returns
    /// The count, if any
    pub fn count(&self) -> Option<u32> {
        self.count
    }

    /// Gets the actions to execute during the transition.
    ///
    /// # Returns
    /// A slice of action functions
    pub fn actions(&self) -> &[Action<S, E>] {
        &self.actions
    }

    /// Gets the guard condition that must be satisfied.
    ///
    /// # Returns
    /// The guard function, if any
    pub fn guard(&self) -> Option<&dyn Fn(&dyn StateContext<S, E>) -> BoxFuture<'static, bool>> {
        self.guard.as_deref()
    }

    /// Gets the type of transition.
    ///
    /// # Returns
    /// The transition kind
    pub fn kind(&self) -> TransitionKind {
        self.kind
    }

    /// Gets the security rule for the transition.
    ///
    /// # Returns
    /// The security rule, if any
    pub fn security_rule(&self) -> Option<&SecurityRule> {
        self.security_rule.as_ref()
    }

    /// Gets the name of the transition.
    ///
    /// # Returns
    /// The transition name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Checks if this is an internal transition (no state change).
    ///
    /// # Returns
    /// `true` if this is an internal transition, `false` otherwise
    pub fn is_internal(&self) -> bool {
        self.kind == TransitionKind::Internal
    }

    /// Checks if this is an external transition (state change).
    ///
    /// # Returns
    /// `true` if this is an external transition, `false` otherwise
    pub fn is_external(&self) -> bool {
        self.kind == TransitionKind::External
    }

    /// Checks if this is a local transition (within same composite state).
    ///
    /// # Returns
    /// `true` if this is a local transition, `false` otherwise
    pub fn is_local(&self) -> bool {
        self.kind == TransitionKind::Local
    }

    /// Checks if the transition has a guard condition.
    ///
    /// # Returns
    /// `true` if the transition has a guard, `false` otherwise
    pub fn has_guard(&self) -> bool {
        self.guard.is_some()
    }

    /// Checks if the transition has actions.
    ///
    /// # Returns
    /// `true` if the transition has actions, `false` otherwise
    pub fn has_actions(&self) -> bool {
        !self.actions.is_empty()
    }

    /// Checks if the transition has a security rule.
    ///
    /// # Returns
    /// `true` if the transition has a security rule, `false` otherwise
    pub fn has_security_rule(&self) -> bool {
        self.security_rule.is_some()
    }

    /// Checks if the transition has a name.
    ///
    /// # Returns
    /// `true` if the transition has a name, `false` otherwise
    pub fn has_name(&self) -> bool {
        !self.name.is_empty()
    }

    /// Checks if this is an event-triggered transition.
    ///
    /// # Returns
    /// `true` if triggered by an event, `false` otherwise
    pub fn is_event_triggered(&self) -> bool {
        self.event.is_some()
    }

    /// Checks if this is a timer-triggered transition.
    ///
    /// # Returns
    /// `true` if triggered by a timer, `false` otherwise
    pub fn is_timer_triggered(&self) -> bool {
        self.period.is_some()
    }

    /// Checks if this is a count-triggered transition.
    ///
    /// # Returns
    /// `true` if triggered by a count, `false` otherwise
    pub fn is_count_triggered(&self) -> bool {
        self.count.is_some()
    }
}

impl<S, E> Default for TransitionData<S, E>
where
    S: Default + Clone + Eq,
    E: Default + Clone + Eq,
{
    fn default() -> Self {
        Self {
            source: S::default(),
            target: S::default(),
            state: None,
            event: Some(E::default()),
            period: None,
            count: None,
            actions: Vec::new(),
            guard: None,
            kind: TransitionKind::External,
            security_rule: None,
            name: String::new(),
        }
    }
}
