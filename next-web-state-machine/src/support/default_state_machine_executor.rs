//! The default executor of a state machine.
//!
//! The Java implementation funnels events through a reactive sink and runs the
//! trigger queue with a concurrency of one. The Rust port keeps the same
//! ordering rules but calls the machine directly, which removes the sink, the
//! context propagation and the per-event subscription allocation.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock, Weak};

use next_web_core::{async_trait, error::BoxError, messaging::Message};
use tracing::debug;

use crate::StateMachine;
use crate::state::StateMachineState;
use crate::state::pseudo_state_kind::PseudoStateKind;
use crate::state_context::StateContext;
use crate::support::lock_util::{lock, read, write};
use crate::support::state_machine_executor::{
    StateMachineExecutor, StateMachineExecutorCallback, StateMachineExecutorTransit,
};
use crate::support::state_machine_interceptor::StateMachineInterceptor;
use crate::support::transition_comparator::TransitionComparator;
use crate::transition::StateMachineTransition;
use crate::transition::transition_conflict_policy::TransitionConflictPolicy;
use crate::trigger::Trigger;
use crate::trigger::default_trigger_context::DefaultTriggerContext;

/// The default executor.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct DefaultStateMachineExecutor<S, E> {
    state_machine: RwLock<Weak<dyn StateMachine<S, E>>>,
    relay: Option<Arc<dyn StateMachine<S, E>>>,
    transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
    triggerless: Vec<Arc<dyn StateMachineTransition<S, E>>>,
    event_index: HashMap<E, Vec<Arc<dyn StateMachineTransition<S, E>>>>,
    triggers: Vec<Arc<dyn Trigger<S, E>>>,
    comparator: TransitionComparator<S, E>,
    conflict_policy: TransitionConflictPolicy,
    defer_list: Mutex<Vec<Arc<dyn Message<E>>>>,
    join_sync_states: Mutex<Vec<Vec<S>>>,
    join_sync_transitions: Mutex<Vec<Arc<dyn StateMachineTransition<S, E>>>>,
    initial_enabled: AtomicBool,
    forwarded_initial_event: RwLock<Option<Arc<dyn Message<E>>>>,
    transit: RwLock<Option<Arc<dyn StateMachineExecutorTransit<S, E>>>>,
    self_weak: RwLock<Weak<DefaultStateMachineExecutor<S, E>>>,
    triggers_registered: AtomicBool,
}

impl<S, E> DefaultStateMachineExecutor<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Eq + Hash + Send + Sync + 'static,
{
    /// Creates a new executor.
    ///
    /// # Arguments
    /// * `state_machine` - a weak handle of the machine which owns the executor
    /// * `relay` - the relay machine, if the machine is a region of another one
    /// * `transitions` - the transitions of the machine
    /// * `conflict_policy` - the policy which resolves competing transitions
    pub fn new(
        state_machine: Weak<dyn StateMachine<S, E>>,
        relay: Option<Arc<dyn StateMachine<S, E>>>,
        transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
        conflict_policy: TransitionConflictPolicy,
    ) -> Self {
        let mut triggerless = Vec::new();
        let mut event_index: HashMap<E, Vec<Arc<dyn StateMachineTransition<S, E>>>> =
            HashMap::new();
        let mut triggers: Vec<Arc<dyn Trigger<S, E>>> = Vec::new();
        let comparator = TransitionComparator::new(conflict_policy);

        for transition in &transitions {
            match transition.trigger() {
                Some(trigger) => {
                    if let Some(event) = trigger.event() {
                        event_index
                            .entry(event.clone())
                            .or_default()
                            .push(transition.clone());
                    }
                    if !triggers.iter().any(|item| Arc::ptr_eq(item, trigger)) {
                        triggers.push(trigger.clone());
                    }
                }
                None => triggerless.push(transition.clone()),
            }
        }

        triggerless.sort_by(|left, right| comparator.compare(left.as_ref(), right.as_ref()));

        Self {
            state_machine: RwLock::new(state_machine),
            relay,
            transitions,
            triggerless,
            event_index,
            triggers,
            comparator,
            conflict_policy,
            defer_list: Mutex::new(Vec::new()),
            join_sync_states: Mutex::new(Vec::new()),
            join_sync_transitions: Mutex::new(Vec::new()),
            initial_enabled: AtomicBool::new(true),
            forwarded_initial_event: RwLock::new(None),
            transit: RwLock::new(None),
            self_weak: RwLock::new(Weak::new()),
            triggers_registered: AtomicBool::new(false),
        }
    }

    /// Gets the state machine of this executor.
    fn state_machine(&self) -> Option<Arc<dyn StateMachine<S, E>>> {
        read(&self.state_machine).upgrade()
    }

    /// Registers the weak handle the trigger listeners call back into.
    ///
    /// # Arguments
    /// * `self_arc` - the shared handle of this executor
    pub fn set_self_arc(self: &Arc<Self>) {
        *write(&self.self_weak) = Arc::downgrade(self);
    }

    /// Registers the executor as listener of every trigger and arms them.
    pub fn arm_triggers(&self) {
        let Some(executor) = read(&self.self_weak).upgrade() else {
            return;
        };

        let first_registration = !self.triggers_registered.swap(true, Ordering::SeqCst);

        for trigger in &self.triggers {
            if first_registration {
                trigger.add_trigger_listener(Arc::new(ExecutorTriggerListener::<S, E> {
                    executor: Arc::downgrade(&executor),
                    trigger: trigger.clone(),
                }));
            }
            trigger.arm();
        }
    }

    /// Disarms every trigger of the machine.
    pub fn disarm_triggers(&self) {
        for trigger in &self.triggers {
            trigger.disarm();
        }
    }

    /// Processes a firing trigger.
    ///
    /// # Arguments
    /// * `trigger` - the trigger which fired
    async fn process_trigger(
        &self,
        trigger: &Arc<dyn Trigger<S, E>>,
    ) -> Result<bool, BoxError> {
        let Some(machine) = self.state_machine() else {
            return Ok(false);
        };
        let Some(current) = machine.state() else {
            return Ok(false);
        };
        let Some(transit) = self.transit_handler() else {
            return Ok(false);
        };

        let candidates: Vec<_> = self
            .transitions
            .iter()
            .filter(|transition| {
                transition
                    .trigger()
                    .map(|candidate| Arc::ptr_eq(candidate, trigger))
                    .unwrap_or(false)
            })
            .filter(|transition| {
                transition
                    .source()
                    .map(|source| current.contains_id(source.id()))
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        for transition in candidates {
            if transit.transit(transition, None).await? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Gets the transit handler.
    fn transit_handler(&self) -> Option<Arc<dyn StateMachineExecutorTransit<S, E>>> {
        read(&self.transit).clone()
    }

    /// Binds the executor to its machine without an executor wrapper.
    ///
    /// # Arguments
    /// * `state_machine` - the machine
    pub fn set_state_machine(&self, state_machine: Arc<dyn StateMachine<S, E>>) {
        *write(&self.state_machine) = Arc::downgrade(&state_machine);
    }

    /// Gets the triggerless transitions, sorted by the comparator.
    pub fn triggerless_transitions(&self) -> &[Arc<dyn StateMachineTransition<S, E>>] {
        &self.triggerless
    }

    /// Gets the transitions of the machine.
    pub fn transitions(&self) -> &[Arc<dyn StateMachineTransition<S, E>>] {
        &self.transitions
    }

    /// Gets the triggers of the machine.
    pub fn triggers(&self) -> &[Arc<dyn Trigger<S, E>>] {
        &self.triggers
    }

    /// Processes one message and reports whether a transition happened.
    ///
    /// # Arguments
    /// * `message` - the message to process
    async fn process_message(
        &self,
        message: Arc<dyn Message<E>>,
    ) -> Result<bool, BoxError> {
        let Some(machine) = self.state_machine() else {
            return Ok(false);
        };

        let Some(current) = machine.state() else {
            return Ok(false);
        };

        let Some(event) = message.get_payload() else {
            return Ok(false);
        };

        let mut candidates = self.candidates(&current, event);
        candidates.sort_by(|left, right| self.comparator.compare(left.as_ref(), right.as_ref()));

        if candidates.is_empty() {
            return Ok(false);
        }

        let Some(transit) = self.transit_handler() else {
            return Ok(false);
        };

        for transition in candidates {
            if let Some(trigger) = transition.trigger() {
                let context = DefaultTriggerContext::<S, E>::new(event);
                if !trigger.evaluate(&context).await {
                    continue;
                }
            }

            if self.is_join_target(&transition) {
                if self.track_join(&transition).await? {
                    return Ok(true);
                }
                continue;
            }

            if transit.transit(transition, Some(message.clone())).await? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Collects the transitions which may react to the event.
    ///
    /// # Arguments
    /// * `current` - the current state
    /// * `event` - the event payload
    fn candidates(
        &self,
        current: &Arc<dyn StateMachineState<S, E>>,
        event: &E,
    ) -> Vec<Arc<dyn StateMachineTransition<S, E>>> {
        let Some(transitions) = self.event_index.get(event) else {
            return Vec::new();
        };

        let mut candidates = Vec::with_capacity(transitions.len());

        // A compound state is addressed by the identifiers of its regions too,
        // so a transition of a nested state wins over one of its parent.
        for transition in transitions {
            let matches = transition
                .source()
                .map(|source| current.contains_id(source.id()))
                .unwrap_or(false);
            if matches {
                candidates.push(transition.clone());
            }
        }

        candidates
    }

    /// Checks whether the transition targets a join pseudo state.
    ///
    /// # Arguments
    /// * `transition` - the transition to inspect
    fn is_join_target(&self, transition: &Arc<dyn StateMachineTransition<S, E>>) -> bool {
        transition
            .target()
            .pseudo_state()
            .map(|pseudo_state| pseudo_state.get_kind() == PseudoStateKind::Join)
            .unwrap_or(false)
    }

    /// Tracks a join and reports whether the join completed.
    ///
    /// # Arguments
    /// * `transition` - the transition which reached the join
    async fn track_join(
        &self,
        transition: &Arc<dyn StateMachineTransition<S, E>>,
    ) -> Result<bool, BoxError> {
        let target = transition.target();

        {
            let mut states = lock(&self.join_sync_states);
            if states.is_empty()
                && let Some(pseudo_state) = target.pseudo_state()
                    && let Some(join) = (pseudo_state.as_ref() as &dyn std::any::Any)
                        .downcast_ref::<crate::state::join_pseudo_state::JoinPseudoState<S, E>>()
                    {
                        for group in join.get_joins() {
                            states.push(group.iter().map(|state| state.id().clone()).collect());
                        }
                    }

            if let Some(source) = transition.source() {
                states.retain(|group| !group.iter().any(|item| item == source.id()));
            }
        }

        lock(&self.join_sync_transitions).push(transition.clone());

        let completed = lock(&self.join_sync_states).is_empty();

        if !completed {
            return Ok(false);
        }

        let transitions = std::mem::take(&mut *lock(&self.join_sync_transitions));
        drop(lock(&self.join_sync_states));

        if let Some(transit) = self.transit_handler() {
            for transition in transitions {
                transit.transit(transition, None).await?;
            }
        }

        Ok(true)
    }

    /// Executes the triggerless transitions until none of them fires.
    ///
    /// # Arguments
    /// * `state` - the state whose transitions are checked
    async fn run_triggerless(
        &self,
        state: &dyn StateMachineState<S, E>,
    ) -> Result<bool, BoxError> {
        let Some(transit) = self.transit_handler() else {
            return Ok(false);
        };

        let mut any = false;

        loop {
            let mut transited = false;

            for transition in &self.triggerless {
                let matches = transition
                    .source()
                    .map(|source| state.contains_id(source.id()))
                    .unwrap_or(false);

                if !matches {
                    continue;
                }

                if transit.transit(transition.clone(), None).await? {
                    transited = true;
                    break;
                }
            }

            if !transited {
                break;
            }

            any = true;
        }

        Ok(any)
    }
}

#[async_trait]
impl<S, E> StateMachineExecutor<S, E> for DefaultStateMachineExecutor<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Eq + Hash + Send + Sync + 'static,
{
    async fn queue_event(
        &self,
        message: Arc<dyn Message<E>>,
        callback: Arc<dyn StateMachineExecutorCallback>,
    ) -> Result<bool, BoxError> {
        let matched = match self.process_message(message).await {
            Ok(matched) => matched,
            Err(error) => {
                callback.error(Box::new(std::io::Error::other(error.to_string())));
                return Err(error);
            }
        };

        // Deferred events are replayed after the new event, and they stay
        // deferred when they still do not match.
        let pending = std::mem::take(&mut *lock(&self.defer_list));
        for deferred in pending {
            let handled = match self.process_message(deferred.clone()).await {
                Ok(handled) => handled,
                Err(error) => {
                    callback.error(error);
                    false
                }
            };

            if !handled {
                lock(&self.defer_list).push(deferred);
            }
        }

        callback.complete();
        Ok(matched)
    }

    fn queue_deferred_event(&self, message: Arc<dyn Message<E>>) {
        debug!("deferring a message");
        lock(&self.defer_list).push(message);
    }

    async fn execute_triggerless_transitions(
        &self,
        _context: &dyn StateContext<S, E>,
        state: &dyn StateMachineState<S, E>,
    ) -> Result<(), BoxError> {
        self.run_triggerless(state).await?;
        Ok(())
    }

    fn set_initial_enabled(&self, enabled: bool) {
        self.initial_enabled.store(enabled, Ordering::SeqCst);
    }

    fn start_triggers(&self) {
        self.arm_triggers();
    }

    fn stop_triggers(&self) {
        self.disarm_triggers();
    }

    fn set_forwarded_initial_event(&self, message: Arc<dyn Message<E>>) {
        *write(&self.forwarded_initial_event) = Some(message);
    }

    fn set_state_machine_executor_transit(
        &self,
        transit_handler: Arc<dyn StateMachineExecutorTransit<S, E>>,
    ) {
        *write(&self.transit) = Some(transit_handler);
    }

    fn add_state_machine_interceptor(&self, interceptor: Arc<dyn StateMachineInterceptor<S, E>>) {
        // The interceptor list lives on the machine, which runs the interceptor
        // callbacks around every transition.
        if let Some(machine) = self.state_machine() {
            let _ = machine.add_state_machine_interceptor(interceptor);
        }
    }
}

impl<S, E> DefaultStateMachineExecutor<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Eq + Hash + Send + Sync + 'static,
{
    /// Checks whether the forwarded initial event is set.
    pub fn forwarded_initial_event(&self) -> Option<Arc<dyn Message<E>>> {
        read(&self.forwarded_initial_event).clone()
    }

    /// Checks whether the initial transition is enabled.
    pub fn is_initial_enabled(&self) -> bool {
        self.initial_enabled.load(Ordering::SeqCst)
    }

    /// Gets the number of deferred events.
    pub fn deferred_len(&self) -> usize {
        lock(&self.defer_list).len()
    }

    /// Gets the relay machine.
    pub fn relay(&self) -> Option<Arc<dyn StateMachine<S, E>>> {
        self.relay.clone()
    }

    /// Gets the conflict policy.
    pub fn conflict_policy(&self) -> TransitionConflictPolicy {
        self.conflict_policy
    }
}

/// Listens to one trigger and lets the executor process its transition.
struct ExecutorTriggerListener<S, E> {
    executor: Weak<DefaultStateMachineExecutor<S, E>>,
    trigger: Arc<dyn Trigger<S, E>>,
}

#[async_trait]
impl<S, E> crate::trigger::trigger_listener::TriggerListener for ExecutorTriggerListener<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Eq + Hash + Send + Sync + 'static,
{
    async fn triggered(&self) {
        let Some(executor) = self.executor.upgrade() else {
            return;
        };

        if let Err(error) = executor.process_trigger(&self.trigger).await {
            tracing::warn!(%error, "a trigger fired but its transition failed");
        }
    }
}
