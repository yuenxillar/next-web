//! Behaviour shared by every state implementation.

use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

use next_web_core::error::BoxError;
use next_web_core::messaging::Message;
use tracing::warn;

use crate::BoxedStateAction;
use crate::action::composite_action_listener::CompositeActionListener;
use crate::action::state_do_action_policy::StateDoActionPolicy;
use crate::region::Region;
use crate::state::action_listener::ActionListener;
use crate::state::composite_state_listener::CompositeStateListener;
use crate::state::pseudo_state::PseudoState;
use crate::state::state_listener::StateListener;
use crate::state_context::StateContext;
use crate::state_machine::StateMachine;
use crate::support::lock_util::{read, write};
use crate::trigger::Trigger;

/// Base implementation of a state.
///
/// This mirrors the Java `AbstractState`: it owns the identifier, the pseudo
/// state, deferred events, entry/exit/state actions, regions, a submachine, the
/// triggers armed while the state is active and the listener registries.
///
/// State actions are executed inline, right after the entry actions, instead of
/// being subscribed to a parallel scheduler. A configured
/// [`StateDoActionPolicy::TimeoutCancel`] bounds each of them with a timeout so
/// a blocking action cannot stall the machine forever.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct AbstractState<S, E> {
    id: S,
    ids: Vec<S>,
    pseudo_state: RwLock<Option<Arc<dyn PseudoState<S, E>>>>,
    deferred: Vec<E>,
    entry_actions: Vec<BoxedStateAction<S, E>>,
    exit_actions: Vec<BoxedStateAction<S, E>>,
    state_actions: Vec<BoxedStateAction<S, E>>,
    regions: Vec<Arc<dyn Region<S, E>>>,
    submachine: Option<Arc<dyn StateMachine<S, E>>>,
    triggers: RwLock<Vec<Arc<dyn Trigger<S, E>>>>,
    state_listener: CompositeStateListener<S, E>,
    action_listener: CompositeActionListener<S, E>,
    state_do_action_policy: Option<StateDoActionPolicy>,
    state_do_action_policy_timeout: Option<Duration>,
}

impl<S, E> AbstractState<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// Creates a new state with the given identifier and pseudo state.
    ///
    /// # Arguments
    /// * `id` - the state identifier
    /// * `pseudo_state` - the pseudo state attached to this state
    pub fn new(id: S, pseudo_state: Option<Arc<dyn PseudoState<S, E>>>) -> Self {
        Self {
            ids: vec![id.clone()],
            id,
            pseudo_state: RwLock::new(pseudo_state),
            deferred: Vec::new(),
            entry_actions: Vec::new(),
            exit_actions: Vec::new(),
            state_actions: Vec::new(),
            regions: Vec::new(),
            submachine: None,
            triggers: RwLock::new(Vec::new()),
            state_listener: CompositeStateListener::new(),
            action_listener: CompositeActionListener::new(),
            state_do_action_policy: None,
            state_do_action_policy_timeout: None,
        }
    }

    /// Creates a new state with an explicit identifier list.
    ///
    /// # Arguments
    /// * `id` - the state identifier
    /// * `ids` - all identifiers of this state, including the ones of its regions
    /// * `pseudo_state` - the pseudo state attached to this state
    pub fn with_ids(id: S, ids: Vec<S>, pseudo_state: Option<Arc<dyn PseudoState<S, E>>>) -> Self {
        let mut state = Self::new(id, pseudo_state);
        if !ids.is_empty() {
            state.ids = ids;
        }
        state
    }

    /// Sets the deferred events of this state.
    ///
    /// # Arguments
    /// * `deferred` - the events which are deferred in this state
    pub fn with_deferred(mut self, deferred: Vec<E>) -> Self {
        self.deferred = deferred;
        self
    }

    /// Sets the entry actions of this state.
    ///
    /// # Arguments
    /// * `actions` - the actions executed when the state is entered
    pub fn with_entry_actions(mut self, actions: Vec<BoxedStateAction<S, E>>) -> Self {
        self.entry_actions = actions;
        self
    }

    /// Sets the exit actions of this state.
    ///
    /// # Arguments
    /// * `actions` - the actions executed when the state is exited
    pub fn with_exit_actions(mut self, actions: Vec<BoxedStateAction<S, E>>) -> Self {
        self.exit_actions = actions;
        self
    }

    /// Sets the state actions of this state.
    ///
    /// # Arguments
    /// * `actions` - the actions executed once, right after entry
    pub fn with_state_actions(mut self, actions: Vec<BoxedStateAction<S, E>>) -> Self {
        self.state_actions = actions;
        self
    }

    /// Sets the regions of this state.
    ///
    /// # Arguments
    /// * `regions` - the orthogonal regions of this state
    pub fn with_regions(mut self, regions: Vec<Arc<dyn Region<S, E>>>) -> Self {
        self.regions = regions;
        self
    }

    /// Sets the submachine of this state.
    ///
    /// # Arguments
    /// * `submachine` - the submachine this state refers to
    pub fn with_submachine(mut self, submachine: Arc<dyn StateMachine<S, E>>) -> Self {
        self.submachine = Some(submachine);
        self
    }

    /// Sets the triggers of this state.
    ///
    /// # Arguments
    /// * `triggers` - the triggers armed while the state is active
    pub fn with_triggers(self, triggers: Vec<Arc<dyn Trigger<S, E>>>) -> Self {
        *write(&self.triggers) = triggers;
        self
    }

    /// Sets the state do action policy.
    ///
    /// # Arguments
    /// * `policy` - the policy applied to state actions
    /// * `timeout` - the timeout used by [`StateDoActionPolicy::TimeoutCancel`]
    pub fn with_state_do_action_policy(
        mut self,
        policy: Option<StateDoActionPolicy>,
        timeout: Option<Duration>,
    ) -> Self {
        self.state_do_action_policy = policy;
        self.state_do_action_policy_timeout = timeout;
        self
    }

    /// Gets the state identifier.
    pub fn id(&self) -> &S {
        &self.id
    }

    /// Gets all identifiers of this state.
    pub fn ids(&self) -> &[S] {
        &self.ids
    }

    /// Gets the pseudo state of this state.
    pub fn pseudo_state(&self) -> Option<Arc<dyn PseudoState<S, E>>> {
        read(&self.pseudo_state).clone()
    }

    /// Attaches a pseudo state to this state.
    ///
    /// # Arguments
    /// * `pseudo_state` - the pseudo state to attach
    pub fn set_pseudo_state(&self, pseudo_state: Option<Arc<dyn PseudoState<S, E>>>) {
        *write(&self.pseudo_state) = pseudo_state;
    }

    /// Gets the deferred events of this state.
    pub fn deferred_events(&self) -> &[E] {
        &self.deferred
    }

    /// Gets the entry actions of this state.
    pub fn entry_actions(&self) -> &[BoxedStateAction<S, E>] {
        &self.entry_actions
    }

    /// Gets the exit actions of this state.
    pub fn exit_actions(&self) -> &[BoxedStateAction<S, E>] {
        &self.exit_actions
    }

    /// Gets the state actions of this state.
    pub fn state_actions(&self) -> &[BoxedStateAction<S, E>] {
        &self.state_actions
    }

    /// Gets the regions of this state.
    pub fn regions(&self) -> &[Arc<dyn Region<S, E>>] {
        &self.regions
    }

    /// Gets the submachine of this state.
    pub fn submachine(&self) -> Option<&Arc<dyn StateMachine<S, E>>> {
        self.submachine.as_ref()
    }

    /// Gets the state listener composite of this state.
    pub fn state_listener(&self) -> &CompositeStateListener<S, E> {
        &self.state_listener
    }

    /// Gets the action listener composite of this state.
    pub fn action_listener(&self) -> &CompositeActionListener<S, E> {
        &self.action_listener
    }

    /// Sets the state do action policy.
    ///
    /// # Arguments
    /// * `policy` - the policy applied to state actions
    /// * `timeout` - the timeout used by [`StateDoActionPolicy::TimeoutCancel`]
    pub fn set_state_do_action_policy(
        &mut self,
        policy: Option<StateDoActionPolicy>,
        timeout: Option<Duration>,
    ) {
        self.state_do_action_policy = policy;
        self.state_do_action_policy_timeout = timeout;
    }

    /// Checks whether the given message is deferred by this state.
    ///
    /// # Arguments
    /// * `message` - the message to check
    pub fn should_defer_message(&self, message: &dyn Message<E>) -> bool
    where
        E: PartialEq,
    {
        match message.get_payload() {
            Some(payload) => self.deferred.iter().any(|event| event == payload),
            None => false,
        }
    }

    /// Adds a state listener.
    ///
    /// # Arguments
    /// * `listener` - the listener to add
    pub fn add_listener(&self, listener: Arc<dyn StateListener<S, E>>) {
        self.state_listener.register(listener);
    }

    /// Removes a state listener.
    ///
    /// # Arguments
    /// * `listener` - the listener to remove
    pub fn remove_listener(&self, listener: Arc<dyn StateListener<S, E>>) {
        self.state_listener.unregister(listener);
    }

    /// Adds an action listener.
    ///
    /// # Arguments
    /// * `listener` - the listener to add
    pub fn add_action_listener_ref(&self, listener: Arc<dyn ActionListener<S, E>>) {
        self.action_listener.register(listener);
    }

    /// Removes an action listener.
    ///
    /// # Arguments
    /// * `listener` - the listener to remove
    pub fn remove_action_listener_ref(&self, listener: Arc<dyn ActionListener<S, E>>) {
        self.action_listener.unregister(listener);
    }

    /// Attaches a trigger which the state arms when it is entered.
    ///
    /// # Arguments
    /// * `trigger` - the trigger to arm on entry
    pub fn add_trigger(&self, trigger: Arc<dyn Trigger<S, E>>) {
        write(&self.triggers).push(trigger);
    }

    /// Arms the triggers of this state.
    pub fn arm_triggers(&self) {
        for trigger in read(&self.triggers).iter() {
            trigger.arm();
        }
    }

    /// Disarms the triggers of this state.
    pub fn disarm_triggers(&self) {
        for trigger in read(&self.triggers).iter() {
            trigger.disarm();
        }
    }

    /// Executes a single action and notifies the action listeners.
    ///
    /// # Arguments
    /// * `action` - the action to execute
    /// * `context` - the state context
    pub async fn execute_action(
        &self,
        action: &BoxedStateAction<S, E>,
        context: &dyn StateContext<S, E>,
    ) -> Result<(), BoxError> {
        let started = std::time::Instant::now();
        action(context).await?;
        let elapsed = started.elapsed();
        if let Some(state_machine) = context.state_machine() {
            self.action_listener
                .notify(state_machine, action, elapsed)
                .await;
            state_machine.notify_action_executed(action, elapsed);
        }
        Ok(())
    }

    /// Executes a list of actions in order.
    ///
    /// # Arguments
    /// * `actions` - the actions to execute
    /// * `context` - the state context
    pub async fn execute_actions(
        &self,
        actions: &[BoxedStateAction<S, E>],
        context: &dyn StateContext<S, E>,
    ) -> Result<(), BoxError> {
        for action in actions {
            self.execute_action(action, context).await?;
        }
        Ok(())
    }

    /// Schedules the state actions.
    ///
    /// State actions run after the entry actions. Their errors are logged and
    /// swallowed, as the Java implementation resumes on error and keeps the
    /// state active.
    ///
    /// # Arguments
    /// * `context` - the state context
    pub async fn schedule_state_actions(&self, context: &dyn StateContext<S, E>) {
        for action in &self.state_actions {
            let started = std::time::Instant::now();
            let result = match self.resolve_state_action_timeout(context) {
                Some(timeout) => if let Ok(result) = tokio::time::timeout(timeout, action(context)).await { result } else {
                    warn!(
                        timeout_ms = timeout.as_millis(),
                        "state action timed out and was cancelled"
                    );
                    continue;
                },
                None => action(context).await,
            };

            if let Err(error) = result {
                warn!(%error, "state action returned an error");
                continue;
            }

            if let Some(state_machine) = context.state_machine() {
                self.action_listener
                    .notify(state_machine, action, started.elapsed())
                    .await;
                state_machine.notify_action_executed(action, started.elapsed());
            }
        }
    }

    fn resolve_state_action_timeout(&self, context: &dyn StateContext<S, E>) -> Option<Duration> {
        if self.state_do_action_policy != Some(StateDoActionPolicy::TimeoutCancel) {
            return None;
        }

        context
            .message_header(crate::support::state_machine_utils::HEADER_DO_ACTION_TIMEOUT)
            .and_then(|value| value.as_i64())
            .map(|millis| Duration::from_millis(millis.max(0) as u64))
            .or(self.state_do_action_policy_timeout)
    }
}
