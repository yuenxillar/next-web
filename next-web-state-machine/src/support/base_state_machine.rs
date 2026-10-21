//! The state machine runtime.
//!
//! This is the port of the Java `AbstractStateMachine`: it owns the states and
//! transitions of one region, processes events and drives the exit and entry
//! sequences.
//!
//! Two things differ from the Java original on purpose. Event handling is
//! serialised with an async mutex instead of a reactive sink, so the pipeline is
//! a straight sequence of `await` calls. And the current state is held as a
//! shared handle instead of a field read, because Rust cannot hand out a
//! reference to a value that another thread may replace.

use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::hash::Hash;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock, Weak};

use next_web_core::async_trait;
use next_web_core::error::BoxError;
use next_web_core::messaging::Message;
use tokio::sync::Mutex as AsyncMutex;
use tracing::debug;
use uuid::Uuid;

use crate::StateMachine;
use crate::access::state_machine_accessor::StateMachineAccessor;
use crate::extended_state::ExtendedState;
use crate::listener::state_machine_listener::StateMachineListener;
use crate::region::Region;
use crate::state::StateMachineState;
use crate::state::pseudo_state::PseudoState;
use crate::state::pseudo_state_kind::PseudoStateKind;
use crate::state_context::{Stage, StateContext};
use crate::state_machine_event_result::{
    DefaultStateMachineEventResult, ResultType, StateMachineEventResult,
};
use crate::state_machine_context::StateMachineContext;
use crate::state_machine_exception::{StateMachineErrorKind, state_machine_error};
use crate::support::default_state_context::DefaultStateContext;
use crate::support::lock_util::{read, write};
use crate::support::state_machine_executor::{
    StateMachineExecutor, StateMachineExecutorTransit,
};
use crate::support::state_machine_interceptor::StateMachineInterceptor;
use crate::support::state_machine_object_support::StateMachineObjectSupport;
use crate::support::transition_comparator::TransitionComparator;
use crate::transition::StateMachineTransition;
use crate::transition::initial_transition::InitialTransition;
use crate::transition::transition_conflict_policy::TransitionConflictPolicy;
use crate::transition::transition_kind::TransitionKind;
use crate::trigger::default_trigger_context::DefaultTriggerContext;

/// The state machine runtime.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct BaseStateMachine<S, E> {
    states: Vec<Arc<dyn StateMachineState<S, E>>>,
    states_by_id: HashMap<S, Arc<dyn StateMachineState<S, E>>>,
    transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
    transitions_by_source: HashMap<S, Vec<Arc<dyn StateMachineTransition<S, E>>>>,
    initial_state: Arc<dyn StateMachineState<S, E>>,
    initial_transition: Arc<dyn StateMachineTransition<S, E>>,
    initial_event: Option<Arc<dyn Message<E>>>,
    extended_state: Arc<dyn ExtendedState>,
    comparator: TransitionComparator<S, E>,
    conflict_policy: TransitionConflictPolicy,
    relay: Option<Arc<dyn StateMachine<S, E>>>,
    parent_machine: Option<Arc<dyn StateMachine<S, E>>>,
    uuid: Uuid,
    id: String,
    current: RwLock<Option<Arc<dyn StateMachineState<S, E>>>>,
    last: RwLock<Option<Arc<dyn StateMachineState<S, E>>>>,
    error: RwLock<Option<String>>,
    running: AtomicBool,
    initial_enabled: AtomicBool,
    history: RwLock<Option<Arc<dyn PseudoState<S, E>>>>,
    forwarded_initial_event: RwLock<Option<Arc<dyn Message<E>>>>,
    executor: RwLock<Option<Arc<dyn StateMachineExecutor<S, E>>>>,
    self_ref: RwLock<Option<Weak<dyn StateMachine<S, E>>>>,
    object_support: StateMachineObjectSupport<S, E>,
    event_lock: AsyncMutex<()>,
    accessor: EmptyAccessor<S, E>,
}

/// A state machine accessor which exposes no region.
struct EmptyAccessor<S, E> {
    _marker: std::marker::PhantomData<(S, E)>,
}

impl<S, E> Default for EmptyAccessor<S, E> {
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S, E> StateMachineAccessor<S, E> for EmptyAccessor<S, E> {
    fn do_with_region(
        &self,
        _state_machine_access: Box<
            dyn Fn(&dyn crate::access::state_machine_access::StateMachineAccess<S, E>),
        >,
    ) {
    }
}

impl<S, E> BaseStateMachine<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// Creates a new state machine.
    ///
    /// # Arguments
    /// * `states` - the states of this region
    /// * `transitions` - the transitions of this region
    /// * `initial_state` - the state the initial transition enters
    /// * `initial_transition` - the transition used on start
    /// * `initial_event` - the event used by the initial transition
    /// * `extended_state` - the extended state of the machine
    pub fn new(
        states: Vec<Arc<dyn StateMachineState<S, E>>>,
        transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
        initial_state: Arc<dyn StateMachineState<S, E>>,
        initial_transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
        initial_event: Option<Arc<dyn Message<E>>>,
        extended_state: Arc<dyn ExtendedState>,
    ) -> Self {
        let initial_transition = initial_transition
            .unwrap_or_else(|| Arc::new(InitialTransition::new(initial_state.clone())));

        Self {
            states,
            states_by_id: HashMap::new(),
            transitions,
            transitions_by_source: HashMap::new(),
            initial_state,
            initial_transition,
            initial_event,
            extended_state,
            comparator: TransitionComparator::new(TransitionConflictPolicy::Child),
            conflict_policy: TransitionConflictPolicy::Child,
            relay: None,
            parent_machine: None,
            uuid: Uuid::new_v4(),
            id: String::new(),
            current: RwLock::new(None),
            last: RwLock::new(None),
            error: RwLock::new(None),
            running: AtomicBool::new(false),
            initial_enabled: AtomicBool::new(true),
            history: RwLock::new(None),
            forwarded_initial_event: RwLock::new(None),
            executor: RwLock::new(None),
            self_ref: RwLock::new(None),
            object_support: StateMachineObjectSupport::default(),
            event_lock: AsyncMutex::new(()),
            accessor: EmptyAccessor::default(),
        }
    }

    /// Wires the machine into its executor and registers the weak self handle
    /// which state contexts carry.
    ///
    /// # Arguments
    /// * `executor` - the executor which processes events of this machine
    pub fn attach(self: &Arc<Self>, executor: Arc<dyn StateMachineExecutor<S, E>>) {
        let me: Arc<dyn StateMachine<S, E>> = self.clone();
        *write(&self.self_ref) = Some(Arc::downgrade(&me));
        executor.set_state_machine_executor_transit(self.clone());
        *write(&self.executor) = Some(executor);
    }

    /// Sets the identifier of this machine.
    ///
    /// # Arguments
    /// * `id` - the identifier
    pub fn set_id(&mut self, id: impl Into<String>) {
        self.id = id.into();
    }

    /// Gets the identifier of this machine.
    pub fn get_id(&self) -> &str {
        &self.id
    }

    /// Sets the relay state machine.
    ///
    /// # Arguments
    /// * `state_machine` - the relay machine
    pub fn set_relay(&mut self, state_machine: Arc<dyn StateMachine<S, E>>) {
        self.relay = Some(state_machine);
    }

    /// Sets the parent machine.
    ///
    /// # Arguments
    /// * `parent_machine` - the parent machine
    pub fn set_parent_machine(&mut self, parent_machine: Arc<dyn StateMachine<S, E>>) {
        self.parent_machine = Some(parent_machine);
    }

    /// Sets the transition conflict policy.
    ///
    /// # Arguments
    /// * `policy` - the conflict policy
    pub fn set_transition_conflict_policy(&mut self, policy: TransitionConflictPolicy) {
        self.conflict_policy = policy;
        self.comparator = TransitionComparator::new(policy);
    }

    /// Sets the event used by the initial transition.
    ///
    /// # Arguments
    /// * `message` - the initial event
    pub fn set_initial_event(&mut self, message: Arc<dyn Message<E>>) {
        self.initial_event = Some(message);
    }

    /// Gets the unique id of this machine.
    pub fn get_uuid(&self) -> Uuid {
        self.uuid
    }

    /// Gets the defined states of this region.
    pub fn get_defined_states(&self) -> &[Arc<dyn StateMachineState<S, E>>] {
        &self.states
    }

    /// Gets the transitions of this region as shared handles.
    pub fn transitions_arc(&self) -> &[Arc<dyn StateMachineTransition<S, E>>] {
        &self.transitions
    }

    /// Gets the current state as a shared handle.
    pub fn current_state_arc(&self) -> Option<Arc<dyn StateMachineState<S, E>>> {
        read(&self.current).clone()
    }

    /// Sets the current state on behalf of the machine which owns this region.
    ///
    /// The parent machine calls this when one of its transitions targets a state
    /// which lives inside this region.
    ///
    /// # Arguments
    /// * `state` - the state to switch to
    /// * `message` - the message which caused the change
    /// * `transition` - the transition behind the change
    /// * `exit` - whether the current state must be exited first
    pub async fn set_current_state_from_parent(
        &self,
        state: Arc<dyn StateMachineState<S, E>>,
        message: Option<Arc<dyn Message<E>>>,
        transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
        exit: bool,
    ) -> Result<(), BoxError> {
        if exit {
            self.exit_current_state(message.clone(), transition.clone())
                .await?;
        }

        if self.is_own_state(&state) {
            if let Some(pseudo_state) = state.pseudo_state()
                && pseudo_state.get_kind() == PseudoStateKind::Join {
                    self.enter_state(state, message, transition).await?;
                    return Ok(());
                }
            self.enter_state(state, message, transition).await
        } else {
            self.switch_to_state(state, message, transition).await
        }
    }

    /// Gets the last state as a shared handle.
    pub fn last_state_arc(&self) -> Option<Arc<dyn StateMachineState<S, E>>> {
        read(&self.last).clone()
    }

    /// Builds the runtime indexes and validates the configuration.
    pub fn init(&mut self) -> Result<(), BoxError> {
        let Some(pseudo_state) = self.initial_state.pseudo_state() else {
            return Err(state_machine_error(
                StateMachineErrorKind::MalformedConfiguration,
                "the initial state must define a pseudo state of kind Initial",
            ));
        };

        if pseudo_state.get_kind() != PseudoStateKind::Initial {
            return Err(state_machine_error(
                StateMachineErrorKind::MalformedConfiguration,
                "the pseudo state of the initial state must be of kind Initial",
            ));
        }

        self.rebuild_index();

        let history = self.states.iter().find_map(|state| {
            state.pseudo_state().and_then(|pseudo_state| {
                matches!(
                    pseudo_state.get_kind(),
                    PseudoStateKind::HistoryDeep | PseudoStateKind::HistoryShallow
                )
                .then(|| pseudo_state.clone())
            })
        });
        *write(&self.history) = history;

        Ok(())
    }

    /// Rebuilds the state and transition indexes.
    fn rebuild_index(&mut self) {
        self.states_by_id.clear();
        self.transitions_by_source.clear();

        for state in &self.states {
            self.states_by_id.insert(state.id().clone(), state.clone());
        }

        for transition in &self.transitions {
            if let Some(source) = transition.source() {
                self.transitions_by_source
                    .entry(source.id().clone())
                    .or_default()
                    .push(transition.clone());
            }
        }
    }

    /// Looks up a state by its identifier.
    ///
    /// # Arguments
    /// * `id` - the state identifier
    fn lookup_state(&self, id: &S) -> Option<Arc<dyn StateMachineState<S, E>>> {
        self.states_by_id.get(id).cloned()
    }

    /// Gets the machine which receives the context notifications.
    fn relay_or_self(&self) -> Option<Arc<dyn StateMachine<S, E>>> {
        if let Some(relay) = self.relay.as_ref() {
            return Some(relay.clone());
        }
        read(&self.self_ref)
            .as_ref()
            .and_then(|weak| weak.upgrade())
    }

    /// Builds a state context.
    ///
    /// # Arguments
    /// * `stage` - the stage of the context
    /// * `message` - the message which caused the context
    /// * `transition` - the transition behind the context
    /// * `source` - the source state
    /// * `target` - the target state
    fn context(
        &self,
        stage: Stage,
        message: Option<Arc<dyn Message<E>>>,
        transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
        source: Option<Arc<dyn StateMachineState<S, E>>>,
        target: Option<Arc<dyn StateMachineState<S, E>>>,
    ) -> DefaultStateContext<S, E> {
        DefaultStateContext::new(
            stage,
            message,
            None,
            Some(self.extended_state.clone()),
            transition,
            self.relay_or_self(),
            source,
            target,
            None,
        )
    }

    /// Checks whether the candidate is a direct state of this region.
    ///
    /// # Arguments
    /// * `candidate` - the state to check
    fn is_own_state(&self, candidate: &Arc<dyn StateMachineState<S, E>>) -> bool {
        self.states_by_id.contains_key(candidate.id())
    }

    /// Checks whether `descendant` lives inside `ancestor`.
    ///
    /// # Arguments
    /// * `ancestor` - the potential ancestor
    /// * `descendant` - the potential descendant
    fn is_descendant(
        &self,
        ancestor: &Arc<dyn StateMachineState<S, E>>,
        descendant: &Arc<dyn StateMachineState<S, E>>,
    ) -> bool {
        crate::support::state_machine_utils::StateMachineUtils::state_contains(
            ancestor,
            descendant.id(),
        )
    }

    /// Collects the transitions which start from the given state.
    ///
    /// # Arguments
    /// * `current` - the current state
    fn candidate_transitions(
        &self,
        current: &Arc<dyn StateMachineState<S, E>>,
    ) -> Vec<Arc<dyn StateMachineTransition<S, E>>> {
        let mut candidates = Vec::new();

        for id in current.ids() {
            let Some(transitions) = self.transitions_by_source.get(id) else {
                continue;
            };

            for transition in transitions {
                if transition.trigger().is_none() {
                    continue;
                }
                if !candidates.iter().any(|item| Arc::ptr_eq(item, transition)) {
                    candidates.push(transition.clone());
                }
            }
        }

        candidates
    }

    /// Checks whether any transition can react to the event.
    ///
    /// # Arguments
    /// * `message` - the message to inspect
    async fn has_matching_trigger(&self, message: &Arc<dyn Message<E>>) -> bool {
        let (Some(current), Some(event)) = (self.current_state_arc(), message.get_payload()) else {
            return false;
        };

        let context = DefaultTriggerContext::<S, E>::new(event);
        for transition in self.candidate_transitions(&current) {
            if let Some(trigger) = transition.trigger()
                && trigger.evaluate(&context).await {
                    return true;
                }
        }

        false
    }

    /// Follows the linked pseudo states of a state.
    ///
    /// # Arguments
    /// * `state` - the state to start from
    /// * `context` - the state context
    async fn follow_linked_pseudo_states(
        &self,
        state: Arc<dyn StateMachineState<S, E>>,
        context: &dyn StateContext<S, E>,
    ) -> Arc<dyn StateMachineState<S, E>> {
        let mut current = state;

        for _ in 0..64 {
            let Some(pseudo_state) = current.pseudo_state() else {
                return current;
            };

            match pseudo_state.get_kind() {
                PseudoStateKind::Initial | PseudoStateKind::Fork => return current,
                _ => match pseudo_state.entry(context).await {
                    Some(next) => current = next,
                    None => return current,
                },
            }
        }

        debug!("stopping pseudo state following after too many hops");
        current
    }

    /// Switches the machine to the given state.
    ///
    /// # Arguments
    /// * `state` - the target state
    /// * `message` - the message which caused the change
    /// * `transition` - the transition behind the change
    async fn switch_to_state(
        &self,
        state: Arc<dyn StateMachineState<S, E>>,
        message: Option<Arc<dyn Message<E>>>,
        transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
    ) -> Result<(), BoxError> {
        self.switch_to_state_inner(state, message, transition, true)
            .await
    }

    /// Switches the machine to the given state.
    ///
    /// # Arguments
    /// * `state` - the target state
    /// * `message` - the message which caused the change
    /// * `transition` - the transition behind the change
    /// * `exit_current` - whether the current state must be exited first
    async fn switch_to_state_inner(
        &self,
        state: Arc<dyn StateMachineState<S, E>>,
        message: Option<Arc<dyn Message<E>>>,
        transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
        exit_current: bool,
    ) -> Result<(), BoxError> {
        let context = self.context(
            Stage::StateChanged,
            message.clone(),
            transition.clone(),
            None,
            Some(state.clone()),
        );

        let target = self.follow_linked_pseudo_states(state, &context).await;

        // Interceptors may veto the state change. Initial transitions and
        // transient pseudo states are not announced, which is what the Java
        // implementation does as well.
        let is_initial = transition
            .as_ref()
            .map(|transition| transition.kind() == TransitionKind::Initial)
            .unwrap_or(false);

        if !is_initial
            && !crate::support::state_machine_utils::StateMachineUtils::is_transient_pseudo_state(
                target.as_ref(),
            )
            && let Some(root) = self.relay_or_self() {
                let allowed = self
                    .object_support
                    .get_state_machine_interceptors()
                    .pre_state_change(
                        target.as_ref(),
                        message.as_deref(),
                        transition.as_deref(),
                        self,
                        root.as_ref(),
                    );

                if !allowed {
                    return Ok(());
                }
            }

        if let Some(pseudo_state) = target.pseudo_state()
            && pseudo_state.get_kind() == PseudoStateKind::Fork {
                if exit_current {
                    self.exit_current_state(message.clone(), transition.clone())
                        .await?;
                }

                let forks = (pseudo_state.as_ref() as &dyn std::any::Any)
                    .downcast_ref::<crate::state::fork_pseudo_state::ForkPseudoState<S, E>>()
                    .map(|fork| fork.get_forks().to_vec())
                    .unwrap_or_default();

                let mut last = None;
                for fork in forks {
                    self.enter_state(fork.clone(), message.clone(), transition.clone())
                        .await?;
                    last = Some(fork);
                }

                if let Some(last) = last {
                    *write(&self.current) = Some(last);
                }

                return Ok(());
            }

        // A state which belongs to a nested region or to the submachine of the
        // current state is entered by that region, not by this machine.
        if !self.is_own_state(&target) {
            if let Some(current) = self.current_state_arc() {
                for region in current.regions() {
                    if region
                        .states()
                        .iter()
                        .any(|state| state.contains_id(target.id()))
                    {
                        return region
                            .set_current_state(target, message, transition, true)
                            .await;
                    }
                }

                if let Some(submachine) = current.submachine()
                    && submachine
                        .states()
                        .iter()
                        .any(|state| state.contains_id(target.id()))
                    {
                        return submachine
                            .set_current_state(target, message, transition, true)
                            .await;
                    }

                // The target belongs to the machine which owns this region, so
                // leave the region and let it enter the target.
                self.exit_current_state(message, transition).await?;
                return Ok(());
            }

            debug!("target state is not part of this region, leaving it to the parent");
            return Ok(());
        }

        if exit_current {
            self.exit_current_state(message.clone(), transition.clone())
                .await?;
        }

        self.enter_state(target, message, transition).await
    }

    /// Exits the current state without clearing it.
    ///
    /// A join leaves its source state while the machine keeps reporting it, so a
    /// later join transition of another source can still match.
    ///
    /// # Arguments
    /// * `message` - the message which caused the transition
    /// * `transition` - the transition
    async fn exit_state_keep_current(
        &self,
        message: Option<Arc<dyn Message<E>>>,
        transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
    ) -> Result<(), BoxError> {
        let Some(current) = self.current_state_arc() else {
            return Ok(());
        };

        let context = self.context(
            Stage::StateExit,
            message,
            transition,
            Some(current.clone()),
            None,
        );
        self.notify_state_exited(&context).await;
        current.exit(&context).await?;
        Ok(())
    }

    /// Tracks a transition which targets a join pseudo state.
    ///
    /// The source state is left, the join is notified about the completed
    /// source, and once every source completed the machine continues with the
    /// state the join pseudo state selects.
    ///
    /// # Arguments
    /// * `join_state` - the state carrying the join pseudo state
    /// * `message` - the message which caused the transition
    /// * `transition` - the transition
    async fn enter_join(
        &self,
        join_state: Arc<dyn StateMachineState<S, E>>,
        message: Option<Arc<dyn Message<E>>>,
        transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
    ) -> Result<(), BoxError> {
        self.exit_state_keep_current(message.clone(), transition.clone())
            .await?;

        let pseudo_state = join_state.pseudo_state();
        let Some(pseudo_state) = pseudo_state else {
            return Ok(());
        };
        let Some(join) = (pseudo_state.as_ref() as &dyn std::any::Any)
            .downcast_ref::<crate::state::join_pseudo_state::JoinPseudoState<S, E>>()
        else {
            return Ok(());
        };

        if let Some(source) = transition.as_ref().and_then(|transition| transition.source()) {
            join.mark_state_completed(source.id());
        }

        if join.is_notified() {
            self.switch_to_state_inner(join_state, message, transition, false)
                .await?;
            join.reset();
        }

        Ok(())
    }

    /// Exits the current state if the transition requires it.
    ///
    /// # Arguments
    /// * `message` - the message which caused the transition
    /// * `transition` - the transition
    async fn exit_current_state(
        &self,
        message: Option<Arc<dyn Message<E>>>,
        transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
    ) -> Result<(), BoxError> {
        let Some(current) = self.current_state_arc() else {
            return Ok(());
        };

        if !self.should_exit(&current, transition.as_ref()) {
            return Ok(());
        }

        let context = self.context(
            Stage::StateExit,
            message.clone(),
            transition.clone(),
            Some(current.clone()),
            None,
        );

        self.notify_state_exited(&context).await;
        current.exit(&context).await?;
        *write(&self.current) = None;

        Ok(())
    }

    /// Decides whether the current state must be exited for a transition.
    ///
    /// # Arguments
    /// * `current` - the current state
    /// * `transition` - the transition, if any
    fn should_exit(
        &self,
        current: &Arc<dyn StateMachineState<S, E>>,
        transition: Option<&Arc<dyn StateMachineTransition<S, E>>>,
    ) -> bool {
        let Some(transition) = transition else {
            return true;
        };

        if transition.kind() == TransitionKind::Internal {
            return false;
        }

        let target = transition.target();

        if transition.kind() == TransitionKind::Local {
            // Local transitions stay inside the current state.
            if self.is_descendant(current, &target) || current.id() == target.id() {
                return false;
            }
            if let Some(source) = transition.source()
                && self.is_descendant(&source, current) {
                    return false;
                }
        }

        // Entering a descendant of the current state keeps the current state
        // active, which is how a compound state keeps its regions running.
        if self.is_descendant(current, &target) {
            return false;
        }

        true
    }

    /// Enters the given state and notifies the listeners.
    ///
    /// # Arguments
    /// * `state` - the state to enter
    /// * `message` - the message which caused the entry
    /// * `transition` - the transition behind the entry
    async fn enter_state(
        &self,
        state: Arc<dyn StateMachineState<S, E>>,
        message: Option<Arc<dyn Message<E>>>,
        transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
    ) -> Result<(), BoxError> {
        // The Java implementation calls the post interceptors here, before the
        // entry sequence, so an anonymous transition cannot reorder them.
        if let Some(root) = self.relay_or_self() {
            self.object_support
                .get_state_machine_interceptors()
                .post_state_change(
                    state.as_ref(),
                    message.as_deref(),
                    transition.as_deref(),
                    self,
                    root.as_ref(),
                );
        }

        let previous = self.current_state_arc();
        let context = self.context(
            Stage::StateEntry,
            message.clone(),
            transition.clone(),
            previous.clone(),
            Some(state.clone()),
        );

        let target_is_join = state
            .pseudo_state()
            .map(|pseudo_state| pseudo_state.get_kind() == PseudoStateKind::Join)
            .unwrap_or(false);

        if !target_is_join {
            self.notify_state_entered(&context).await;
        }

        *write(&self.current) = Some(state.clone());
        state.entry(&context).await?;

        if !target_is_join {
            let changed = self.context(
                Stage::StateChanged,
                message,
                transition,
                previous,
                Some(state.clone()),
            );
            self.notify_state_changed(&changed).await;
        }

        Ok(())
    }

    /// Records the history state after a transition.
    ///
    /// # Arguments
    /// * `transition` - the transition which ran
    fn record_history(&self, transition: &Arc<dyn StateMachineTransition<S, E>>) {
        if transition.kind() == TransitionKind::Initial {
            return;
        }

        let history = read(&self.history).clone();
        let Some(history) = history else {
            return;
        };

        let Some(current) = self.current_state_arc() else {
            return;
        };

        if let Some(history_state) = (history.as_ref() as &dyn std::any::Any)
            .downcast_ref::<crate::state::history_pseudo_state::HistoryPseudoState<S, E>>()
        {
            match history.get_kind() {
                PseudoStateKind::HistoryShallow | PseudoStateKind::HistoryDeep => {
                    history_state.set_state(current);
                }
                _ => {}
            }
        }
    }

    /// Starts the machine if it is not running yet.
    async fn ensure_started(&self) -> Result<(), BoxError> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }
        self.start().await
    }

    /// Sends the initial event through the initial transition.
    async fn start_initial(&self) -> Result<(), BoxError> {
        let forwarded = read(&self.forwarded_initial_event).clone();
        let initial_event = forwarded.or_else(|| self.initial_event.clone());

        let context = self.context(
            Stage::Transition,
            initial_event.clone(),
            Some(self.initial_transition.clone()),
            None,
            Some(self.initial_state.clone()),
        );

        self.initial_transition
            .execute_transition_actions(&context)
            .await?;

        self.switch_to_state(
            self.initial_transition.target(),
            initial_event.clone(),
            Some(self.initial_transition.clone()),
        )
        .await?;

        let started = self.context(
            Stage::StateMachineStart,
            initial_event,
            Some(self.initial_transition.clone()),
            None,
            self.current_state_arc(),
        );
        self.notify_state_machine_started(&started).await;

        Ok(())
    }
}

#[async_trait]
impl<S, E> Region<S, E> for BaseStateMachine<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn id(&self) -> &str {
        &self.id
    }

    fn get_uuid(&self) -> Uuid {
        self.uuid
    }

    async fn start(&self) -> Result<(), BoxError> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        if self.initial_enabled.load(Ordering::SeqCst) {
            self.start_initial().await?;
        }

        // Triggers are armed after the initial transition, so a timer cannot
        // fire before the machine has a current state.
        let executor = read(&self.executor).clone();
        if let Some(executor) = executor {
            executor.start_triggers();
        }

        Ok(())
    }

    async fn stop(&self) -> Result<(), BoxError> {
        if !self.running.swap(false, Ordering::SeqCst) {
            return Ok(());
        }

        let executor = read(&self.executor).clone();
        if let Some(executor) = executor {
            executor.stop_triggers();
        }

        let context = self.context(
            Stage::StateMachineStop,
            None,
            None,
            self.current_state_arc(),
            None,
        );
        self.notify_state_machine_stopped(&context).await;

        if let Some(current) = self.current_state_arc() {
            *write(&self.last) = Some(current);
        }
        *write(&self.current) = None;

        Ok(())
    }

    async fn set_current_state(
        &self,
        state: Arc<dyn StateMachineState<S, E>>,
        message: Option<Arc<dyn Message<E>>>,
        transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
        exit: bool,
    ) -> Result<(), BoxError> {
        self.set_current_state_from_parent(state, message, transition, exit)
            .await
    }

    async fn send_event(
        &self,
        event: Arc<dyn Message<E>>,
    ) -> Result<Arc<dyn StateMachineEventResult<S, E>>, BoxError> {
        // One event at a time: the pipeline mutates the current state and runs
        // user actions, so concurrent events would interleave the entry and
        // exit sequences.
        let _guard = self.event_lock.lock().await;

        if self.has_state_machine_error() {
            return Ok(Arc::new(DefaultStateMachineEventResult::denied(
                None, event,
            )));
        }

        let Some(event) = self
            .object_support
            .get_state_machine_interceptors()
            .pre_event(event.clone(), self)
        else {
            return Ok(Arc::new(DefaultStateMachineEventResult::denied(
                None, event,
            )));
        };

        let Some(current) = self.current_state_arc() else {
            return Ok(Arc::new(DefaultStateMachineEventResult::denied(
                None, event,
            )));
        };

        if current.should_defer(event.as_ref()) {
            if let Some(executor) = read(&self.executor).clone() {
                executor.queue_deferred_event(event.clone());
            }
            return Ok(Arc::new(DefaultStateMachineEventResult::deferred(
                None, event,
            )));
        }

        // A state which owns a region processes the event itself.
        let state_results = current.send_event(event.clone()).await;
        if state_results
            .iter()
            .any(|result| result.result_type() == ResultType::Accepted)
        {
            return Ok(state_results.into_iter().next().unwrap_or_else(|| {
                Arc::new(DefaultStateMachineEventResult::accepted(None, event))
            }));
        }

        if !self.has_matching_trigger(&event).await {
            let context = self.context(
                Stage::EventNotAccepted,
                Some(event.clone()),
                None,
                self.current_state_arc(),
                None,
            );
            self.notify_event_not_accepted(&context).await;
            return Ok(Arc::new(DefaultStateMachineEventResult::denied(
                None, event,
            )));
        }

        let Some(executor) = read(&self.executor).clone() else {
            return Ok(Arc::new(DefaultStateMachineEventResult::denied(
                None, event,
            )));
        };

        let callback = Arc::new(
            crate::support::state_machine_executor::DefaultStateMachineExecutorCallback::new(),
        );

        let matched = executor.queue_event(event.clone(), callback.clone()).await;

        let matched = match matched {
            Ok(matched) => matched,
            Err(error) => {
                let mut error_context = self.context(
                    Stage::StateMachineError,
                    Some(event.clone()),
                    None,
                    None,
                    None,
                );
                error_context.set_error(error);
                self.notify_error(&error_context).await;
                return Ok(Arc::new(DefaultStateMachineEventResult::denied(
                    None, event,
                )));
            }
        };

        if !matched {
            let context = self.context(
                Stage::EventNotAccepted,
                Some(event.clone()),
                None,
                self.current_state_arc(),
                None,
            );
            self.notify_event_not_accepted(&context).await;
            return Ok(Arc::new(DefaultStateMachineEventResult::denied(
                None, event,
            )));
        }

        if let Some(current) = self.current_state_arc() {
            let context = self.context(
                Stage::StateChanged,
                Some(event.clone()),
                None,
                None,
                Some(current.clone()),
            );
            self.execute_triggerless_transitions(&context, current.as_ref())
                .await?;
        }

        Ok(Arc::new(DefaultStateMachineEventResult::accepted(
            None, event,
        )))
    }

    async fn send_events(
        &self,
        events: Vec<Arc<dyn Message<E>>>,
    ) -> Result<Vec<Arc<dyn StateMachineEventResult<S, E>>>, BoxError> {
        let mut results = Vec::with_capacity(events.len());
        for event in events {
            results.push(self.send_event(event).await?);
        }
        Ok(results)
    }

    fn state(&self) -> Option<Arc<dyn StateMachineState<S, E>>> {
        // Once a machine stopped, the state it left is still reported while it
        // is an end state, which is what the Java implementation does.
        if let Some(current) = self.current_state_arc() {
            return Some(current);
        }

        let last = self.last_state_arc()?;
        let is_end = last
            .pseudo_state()
            .map(|pseudo_state| pseudo_state.get_kind() == PseudoStateKind::End)
            .unwrap_or(false);

        is_end.then_some(last)
    }

    fn states(&self) -> Vec<Arc<dyn StateMachineState<S, E>>> {
        self.states.clone()
    }

    fn transitions(&self) -> Vec<&dyn StateMachineTransition<S, E>> {
        self.transitions.iter().map(AsRef::as_ref).collect()
    }

    fn is_complete(&self) -> bool {
        match self.current_state_arc() {
            Some(state) => state
                .pseudo_state()
                .map(|pseudo_state| pseudo_state.get_kind() == PseudoStateKind::End)
                .unwrap_or(false),
            // A machine which is not running and has no current state anymore
            // has completed, which is what the Java implementation reports.
            None => !self.running.load(Ordering::SeqCst),
        }
    }

    fn set_forwarded_initial_event(&self, message: Arc<dyn Message<E>>) {
        *write(&self.forwarded_initial_event) = Some(message);
    }

    fn set_initial_enabled(&self, enabled: bool) {
        self.initial_enabled.store(enabled, Ordering::SeqCst);
    }

    fn add_state_listener(
        &self,
        listener: Arc<dyn StateMachineListener<S, E>>,
    ) -> Result<(), BoxError> {
        self.object_support.get_state_listener().register(listener);
        Ok(())
    }

    fn remove_state_listener(
        &self,
        listener: Arc<dyn StateMachineListener<S, E>>,
    ) -> Result<(), BoxError> {
        self.object_support
            .get_state_listener()
            .unregister(listener.as_ref());
        Ok(())
    }
}

impl<S, E> StateMachine<S, E> for BaseStateMachine<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn initial_state(&self) -> &dyn StateMachineState<S, E> {
        self.initial_state.as_ref()
    }

    fn extended_state(&self) -> &dyn ExtendedState {
        self.extended_state.as_ref()
    }

    fn state_machine_accessor(&self) -> &dyn StateMachineAccessor<S, E> {
        &self.accessor
    }

    fn set_state_machine_error(&self, error: BoxError) {
        let intercepted = self
            .object_support
            .get_state_machine_interceptors()
            .state_machine_error(self, error)
            .map(|error| error.to_string());

        *write(&self.error) = intercepted;
    }

    fn has_state_machine_error(&self) -> bool {
        read(&self.error).is_some()
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn add_state_machine_interceptor(
        &self,
        interceptor: Arc<dyn StateMachineInterceptor<S, E>>,
    ) -> Result<(), BoxError> {
        self.object_support
            .get_state_machine_interceptors()
            .add(interceptor);
        Ok(())
    }

    fn add_state_machine_monitor(
        &self,
        monitor: Arc<dyn crate::monitor::state_machine_monitor::StateMachineMonitor<S, E>>,
    ) -> Result<(), BoxError> {
        self.object_support
            .get_state_machine_monitor()
            .register(monitor);
        Ok(())
    }

    fn notify_action_executed(
        &self,
        action: &crate::BoxedStateAction<S, E>,
        duration: std::time::Duration,
    ) {
        if let Some(state_machine) = self.relay_or_self() {
            self.object_support
                .notify_action_monitor(state_machine.as_ref(), action, duration);
        }
    }

    fn get_history_state(&self) -> Option<Arc<dyn PseudoState<S, E>>> {
        read(&self.history).clone()
    }

    fn snapshot_context(
        &self,
    ) -> Result<Arc<dyn StateMachineContext<S, E>>, BoxError> {
        let Some(current) = self.current_state_arc() else {
            return Err(state_machine_error(
                StateMachineErrorKind::NotStarted,
                "the machine has no state to persist",
            ));
        };

        let extended_state: Arc<dyn ExtendedState> = Arc::new(
            crate::support::default_extended_state::DefaultExtendedState::with_variables(
                self.extended_state.get_variables().clone(),
            ),
        );

        Ok(Arc::new(
            crate::support::default_state_machine_context::DefaultStateMachineContext::new(
                self.id.clone(),
                current.id().clone(),
            )
            .with_extended_state(extended_state),
        ))
    }

    fn reset_to(&self, state: &S) -> Result<(), BoxError> {
        let Some(target) = self.lookup_state(state) else {
            return Err(state_machine_error(
                StateMachineErrorKind::StateNotFound,
                "the state to restore was not found in the machine",
            ));
        };

        *write(&self.error) = None;
        *write(&self.current) = Some(target);

        Ok(())
    }
}

#[async_trait]
impl<S, E> StateMachineExecutorTransit<S, E> for BaseStateMachine<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    async fn transit(
        &self,
        transition: Arc<dyn StateMachineTransition<S, E>>,
        message: Option<Arc<dyn Message<E>>>,
    ) -> Result<bool, BoxError> {
        let started = std::time::Instant::now();
        let context = self.context(
            Stage::Transition,
            message.clone(),
            Some(transition.clone()),
            None,
            Some(transition.target()),
        );

        if !self
            .object_support
            .get_state_machine_interceptors()
            .pre_transition(&context)
        {
            return Ok(false);
        }

        if !transition.transit(&context).await {
            return Ok(false);
        }

        let start_context = self.context(
            Stage::TransitionStart,
            message.clone(),
            Some(transition.clone()),
            None,
            None,
        );
        self.notify_transition_start(&start_context).await;

        transition.execute_transition_actions(&context).await?;

        self.notify_transition(&context).await;

        let target = transition.target();
        let is_join = target
            .pseudo_state()
            .map(|pseudo_state| pseudo_state.get_kind() == PseudoStateKind::Join)
            .unwrap_or(false);

        if is_join {
            self.enter_join(target, message.clone(), Some(transition.clone()))
                .await?;
        } else {
            match transition.kind() {
                TransitionKind::Initial => {
                    self.switch_to_state(target, message.clone(), Some(transition.clone()))
                        .await?;
                    let started = self.context(
                        Stage::StateMachineStart,
                        message.clone(),
                        Some(transition.clone()),
                        None,
                        self.current_state_arc(),
                    );
                    self.notify_state_machine_started(&started).await;
                }
                TransitionKind::Internal => {}
                _ => {
                    self.switch_to_state(target, message.clone(), Some(transition.clone()))
                        .await?;
                }
            }
        }

        self.record_history(&transition);

        let end_context = self.context(
            Stage::TransitionEnd,
            message,
            Some(transition.clone()),
            None,
            None,
        );
        self.notify_transition_end(&end_context).await;
        self.object_support
            .get_state_machine_interceptors()
            .post_transition(&context);

        if let Some(state_machine) = self.relay_or_self() {
            self.object_support.notify_transition_monitor(
                state_machine.as_ref(),
                transition.as_ref(),
                started.elapsed(),
            );
        }

        if self.is_complete() {
            self.stop().await?;
        }

        Ok(true)
    }
}

impl<S, E> BaseStateMachine<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// Executes the triggerless transitions of the current state.
    ///
    /// # Arguments
    /// * `context` - the state context
    /// * `state` - the state to start from
    pub async fn execute_triggerless_transitions(
        &self,
        context: &dyn StateContext<S, E>,
        state: &dyn StateMachineState<S, E>,
    ) -> Result<(), BoxError> {
        let executor = read(&self.executor).clone();
        if let Some(executor) = executor {
            executor
                .execute_triggerless_transitions(context, state)
                .await?;
        }
        Ok(())
    }

    /// Notifies the listeners that the machine entered an error.
    ///
    /// # Arguments
    /// * `context` - the context carrying the error
    pub async fn notify_error(&self, context: &dyn StateContext<S, E>) {
        self.object_support.notify_state_machine_error(context).await;
    }

    /// Gets the transition conflict policy.
    pub fn transition_conflict_policy(&self) -> TransitionConflictPolicy {
        self.conflict_policy
    }

    /// Gets the comparator of transitions.
    pub fn comparator(&self) -> &TransitionComparator<S, E> {
        &self.comparator
    }

    /// Gets the executor of this machine.
    pub fn executor(&self) -> Option<Arc<dyn StateMachineExecutor<S, E>>> {
        read(&self.executor).clone()
    }

    /// Checks whether the machine owns the given state.
    ///
    /// # Arguments
    /// * `id` - the state identifier
    pub fn owns_state(&self, id: &S) -> bool {
        self.states_by_id.contains_key(id)
    }

    /// Gets the parent machine.
    pub fn parent_machine(&self) -> Option<Arc<dyn StateMachine<S, E>>> {
        self.parent_machine.clone()
    }

    /// Gets the relay machine.
    pub fn relay_machine(&self) -> Option<Arc<dyn StateMachine<S, E>>> {
        self.relay.clone()
    }

    /// Looks up one of the defined states by identifier.
    ///
    /// # Arguments
    /// * `id` - the state identifier
    pub fn state_by_id(&self, id: &S) -> Option<Arc<dyn StateMachineState<S, E>>> {
        self.lookup_state(id)
    }

    /// Starts the machine if it is not running yet, used when a transition
    /// arrives before the machine was started.
    pub async fn start_if_needed(&self) -> Result<(), BoxError> {
        self.ensure_started().await
    }
}

impl<S, E> fmt::Display for BaseStateMachine<S, E>
where
    S: Debug + Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buffer = String::new();

        for state in &self.states {
            buffer.push_str(&format!("{:?} ", state.id()));
        }

        buffer.push_str(" / ");
        if let Some(current) = self.current_state_arc() {
            buffer.push_str(&format!("{:?}", current.ids()));
        }

        buffer.push_str(" / uuid=");
        buffer.push_str(&self.uuid.to_string());

        write!(f, "{buffer}")
    }
}

impl<S, E> Debug for BaseStateMachine<S, E>
where
    S: Debug + Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BaseStateMachine")
            .field("uuid", &self.uuid)
            .field(
                "current",
                &self.current_state_arc().map(|state| state.id().clone()),
            )
            .finish()
    }
}

impl<S, E> Deref for BaseStateMachine<S, E> {
    type Target = StateMachineObjectSupport<S, E>;

    fn deref(&self) -> &Self::Target {
        &self.object_support
    }
}

impl<S, E> DerefMut for BaseStateMachine<S, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.object_support
    }
}
