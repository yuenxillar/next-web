use next_web_core::async_trait;
use next_web_core::error::BoxError;
use next_web_core::messaging::message_headers::MessageHeaders;
use next_web_core::traits::message::Message;
use std::fmt::{self, Debug};
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};
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
use crate::state_machine_event_result::StateMachineEventResult;
use crate::support::default_extended_state::DefaultExtendedState;
use crate::support::default_state_context::DefaultStateContext;
use crate::support::lifecycle_object_support::LifecycleObjectSupportExt;
use crate::support::state_machine_executor::StateMachineExecutor;
use crate::support::state_machine_object_support::{
    StateMachineObjectSupport, StateMachineObjectSupportExt,
};
use crate::transition::StateMachineTransition;
use crate::transition::initial_transition::InitialTransition;
use crate::transition::transition_conflict_policy::TransitionConflictPolicy;
use crate::transition::transition_kind::TransitionKind;
use crate::trigger::trigger_context::TriggerContext;

fn state_machine_error(message: impl Into<String>) -> BoxError {
    std::io::Error::other(message.into()).into()
}

struct EmptyStateMachineAccessor<S, E> {
    _marker: PhantomData<(S, E)>,
}

impl<S, E> Default for EmptyStateMachineAccessor<S, E> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<S, E> StateMachineAccessor<S, E> for EmptyStateMachineAccessor<S, E> {
    fn do_with_region(
        &self,
        _state_machine_access: Box<
            dyn Fn(&dyn crate::access::state_machine_access::StateMachineAccess<S, E>) -> (),
        >,
    ) {
    }
}

struct SimpleTriggerContext<'a, E> {
    event: &'a E,
}

impl<'a, S, E> TriggerContext<S, E> for SimpleTriggerContext<'a, E>
where
    E: Send + Sync,
{
    fn get_event(&self) -> &E {
        self.event
    }
}

pub struct BaseStateMachine<S, E> {
    states: Vec<Arc<dyn StateMachineState<S, E>>>,
    transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
    initial_state: Arc<dyn StateMachineState<S, E>>,
    initial_transition: Arc<dyn StateMachineTransition<S, E>>,
    initial_event: Option<Box<dyn Message<E>>>,
    extended_state: Arc<dyn ExtendedState>,
    transition_conflict_policy: Option<TransitionConflictPolicy>,
    current_state_id: RwLock<Option<S>>,
    last_state_id: RwLock<Option<S>>,
    current_error: Option<BoxError>,
    history: Option<Arc<dyn PseudoState<S, E>>>,
    trigger_to_transition: Vec<(String, Arc<dyn StateMachineTransition<S, E>>)>,
    triggerless_transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
    relay: Option<Arc<dyn StateMachine<S, E>>>,
    state_machine_executor: Option<Arc<dyn StateMachineExecutor<S, E>>>,
    initial_enabled: Option<bool>,
    uuid: Uuid,
    id: Option<String>,
    forwarded_initial_event: Option<Box<dyn Message<E>>>,
    parent_machine: Option<Arc<dyn StateMachine<S, E>>>,
    accessor: EmptyStateMachineAccessor<S, E>,
    object_support: StateMachineObjectSupport<S, E>,
}

impl<S, E> BaseStateMachine<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    pub fn new(
        states: Vec<Arc<dyn StateMachineState<S, E>>>,
        transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
        initial_state: Arc<dyn StateMachineState<S, E>>,
    ) -> Self {
        Self::with_extended_state(
            states,
            transitions,
            initial_state,
            Arc::new(DefaultExtendedState::default()),
        )
    }

    pub fn with_extended_state(
        states: Vec<Arc<dyn StateMachineState<S, E>>>,
        transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
        initial_state: Arc<dyn StateMachineState<S, E>>,
        extended_state: Arc<dyn ExtendedState>,
    ) -> Self {
        Self::with_all(
            states,
            transitions,
            initial_state,
            None,
            None,
            extended_state,
            None,
        )
    }

    pub fn with_all(
        states: Vec<Arc<dyn StateMachineState<S, E>>>,
        transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
        initial_state: Arc<dyn StateMachineState<S, E>>,
        initial_transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
        initial_event: Option<Box<dyn Message<E>>>,
        extended_state: Arc<dyn ExtendedState>,
        uuid: Option<Uuid>,
    ) -> Self {
        let initial_transition = initial_transition
            .unwrap_or_else(|| Arc::new(InitialTransition::new(initial_state.clone())));

        Self {
            states,
            transitions,
            initial_state,
            initial_transition,
            initial_event,
            extended_state,
            transition_conflict_policy: None,
            current_state_id: RwLock::new(None),
            last_state_id: RwLock::new(None),
            current_error: None,
            history: None,
            trigger_to_transition: Vec::new(),
            triggerless_transitions: Vec::new(),
            relay: None,
            state_machine_executor: None,
            initial_enabled: None,
            uuid: uuid.unwrap_or_else(Uuid::new_v4),
            id: None,
            forwarded_initial_event: None,
            parent_machine: None,
            accessor: EmptyStateMachineAccessor::default(),
            object_support: Default::default(),
        }
    }

    pub fn set_transition_conflict_policy(
        &mut self,
        transition_conflict_policy: TransitionConflictPolicy,
    ) {
        self.transition_conflict_policy = Some(transition_conflict_policy);
    }

    pub fn set_state_machine_executor(
        &mut self,
        state_machine_executor: Arc<dyn StateMachineExecutor<S, E>>,
    ) {
        self.state_machine_executor = Some(state_machine_executor);
    }

    pub fn set_relay(&mut self, state_machine: Arc<dyn StateMachine<S, E>>) {
        self.relay = Some(state_machine);
    }

    pub fn set_parent_machine(&mut self, parent_machine: Arc<dyn StateMachine<S, E>>) {
        self.parent_machine = Some(parent_machine);
    }

    pub fn set_forwarded_initial_event(&mut self, message: Box<dyn Message<E>>) {
        self.forwarded_initial_event = Some(message);
    }

    pub fn set_initial_enabled(&mut self, enabled: bool) {
        self.initial_enabled = Some(enabled);
    }

    pub fn get_uuid(&self) -> Uuid {
        self.uuid
    }

    pub fn get_id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub fn set_id(&mut self, id: impl Into<String>) {
        self.id = Some(id.into());
    }
}

impl<S, E> BaseStateMachine<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    pub fn set_history_state(&mut self, history: Arc<dyn PseudoState<S, E>>) {
        self.history = Some(history);
    }

    pub fn get_history_state(&self) -> Option<&dyn PseudoState<S, E>> {
        self.history.as_deref()
    }

    fn relay_state_machine(&self) -> Option<Arc<dyn StateMachine<S, E>>> {
        self.relay.clone()
    }

    fn current_state_id(&self) -> Option<S> {
        self.current_state_id
            .read()
            .expect("current_state_id poisoned")
            .clone()
    }

    fn set_current_state_id(&self, state_id: Option<S>) {
        *self
            .current_state_id
            .write()
            .expect("current_state_id poisoned") = state_id;
    }

    fn current_state_arc(&self) -> Option<Arc<dyn StateMachineState<S, E>>> {
        self.current_state_id()
            .and_then(|state_id| self.find_state_arc_by_id(&state_id))
    }

    fn last_state_id(&self) -> Option<S> {
        self.last_state_id
            .read()
            .expect("last_state_id poisoned")
            .clone()
    }

    fn set_last_state_id(&self, state_id: Option<S>) {
        *self.last_state_id.write().expect("last_state_id poisoned") = state_id;
    }

    fn find_state_arc_by_id(&self, state_id: &S) -> Option<Arc<dyn StateMachineState<S, E>>> {
        self.states
            .iter()
            .find(|state| state.id() == state_id)
            .cloned()
    }

    fn find_state_ref_by_id(&self, state_id: &S) -> Option<&dyn StateMachineState<S, E>> {
        self.states
            .iter()
            .find(|state| state.id() == state_id)
            .map(|state| state.as_ref())
    }

    fn build_state_context(
        &self,
        stage: Stage,
        message: Option<Box<dyn Message<E>>>,
        transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
        state_machine: Option<Arc<dyn StateMachine<S, E>>>,
        source: Option<Arc<dyn StateMachineState<S, E>>>,
        target: Option<Arc<dyn StateMachineState<S, E>>>,
        error: Option<BoxError>,
    ) -> DefaultStateContext<S, E> {
        let message_headers = match message.as_ref() {
            Some(message) => Some(message.get_headers().clone()),
            None => Some(MessageHeaders::default()),
        };

        DefaultStateContext::new(
            stage,
            message,
            message_headers,
            Some(self.extended_state.clone()),
            transition,
            state_machine,
            source,
            target,
            error,
        )
    }

    async fn follow_linked_pseudo_states(
        &self,
        state: Arc<dyn StateMachineState<S, E>>,
        message: Option<Box<dyn Message<E>>>,
        transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
    ) -> Arc<dyn StateMachineState<S, E>> {
        let mut current_state = state;
        let mut current_message = message;
        let mut current_transition = transition;

        loop {
            let Some(pseudo_state) = current_state.pseudo_state() else {
                return current_state;
            };

            match pseudo_state.get_kind() {
                PseudoStateKind::Initial | PseudoStateKind::Fork => return current_state,
                _ => {
                    let context = self.build_state_context(
                        Stage::StateEntry,
                        current_message,
                        current_transition,
                        self.relay_state_machine(),
                        None,
                        Some(current_state.clone()),
                        None,
                    );

                    match pseudo_state.entry(&context).await {
                        Some(next_state) => {
                            current_state = next_state;
                            current_message = None;
                            current_transition = None;
                        }
                        None => return current_state,
                    }
                }
            }
        }
    }

    async fn find_matching_transition(
        &self,
        current_state: &dyn StateMachineState<S, E>,
        event: &dyn Message<E>,
    ) -> Option<Arc<dyn StateMachineTransition<S, E>>> {
        let trigger_event = event.get_payload()?;
        let trigger_context = SimpleTriggerContext {
            event: trigger_event,
        };

        for transition in &self.transitions {
            if transition.kind() == TransitionKind::Initial {
                continue;
            }

            if transition.source().id() != current_state.id() {
                continue;
            }

            let Some(trigger) = transition.trigger() else {
                continue;
            };

            if !trigger.evaluate(&trigger_context).await {
                continue;
            }

            let context = self.build_state_context(
                Stage::Transition,
                None,
                Some(transition.clone()),
                self.relay_state_machine(),
                self.find_state_arc_by_id(current_state.id()),
                self.find_state_arc_by_id(transition.target().id()),
                None,
            );

            if transition.transit(&context).await {
                return Some(transition.clone());
            }
        }

        None
    }

    async fn apply_transition(
        &self,
        transition: Arc<dyn StateMachineTransition<S, E>>,
        message: Option<Box<dyn Message<E>>>,
    ) -> Result<(), BoxError> {
        let source_state = if transition.kind() == TransitionKind::Initial {
            None
        } else {
            self.current_state_arc()
        };

        let target_state = self
            .find_state_arc_by_id(transition.target().id())
            .ok_or_else(|| {
                state_machine_error("transition target state was not found in this machine")
            })?;

        let transition_context = self.build_state_context(
            Stage::TransitionStart,
            message.clone(),
            Some(transition.clone()),
            self.relay_state_machine(),
            source_state.clone(),
            Some(target_state.clone()),
            None,
        );
        self.object_support
            .notify_transition_start(&transition_context)
            .await;

        transition
            .execute_transition_actions(&transition_context)
            .await;

        self.object_support
            .notify_transition(&transition_context)
            .await;

        if transition.kind() != TransitionKind::Internal {
            if let Some(source_state) = source_state.as_ref() {
                let exit_context = self.build_state_context(
                    Stage::StateExit,
                    message.clone(),
                    Some(transition.clone()),
                    self.relay_state_machine(),
                    Some(source_state.clone()),
                    None,
                    None,
                );
                self.object_support.notify_state_exited(&exit_context).await;
                source_state.exit(&exit_context).await;
            }

            let resolved_target = self
                .follow_linked_pseudo_states(
                    target_state.clone(),
                    message.clone(),
                    Some(transition.clone()),
                )
                .await;

            self.set_current_state_id(Some(resolved_target.id().clone()));

            let entry_context = self.build_state_context(
                Stage::StateEntry,
                message.clone(),
                Some(transition.clone()),
                self.relay_state_machine(),
                source_state.clone(),
                self.find_state_arc_by_id(resolved_target.id()),
                None,
            );
            self.object_support
                .notify_state_entered(&entry_context)
                .await;
            resolved_target.entry(&entry_context).await;

            if source_state.is_some() {
                let changed_context = self.build_state_context(
                    Stage::StateChanged,
                    message.clone(),
                    Some(transition.clone()),
                    self.relay_state_machine(),
                    source_state,
                    self.find_state_arc_by_id(resolved_target.id()),
                    None,
                );
                self.object_support
                    .notify_state_changed(&changed_context)
                    .await;
            }
        }

        let transition_end_context = self.build_state_context(
            Stage::TransitionEnd,
            message,
            Some(transition),
            self.relay_state_machine(),
            None,
            None,
            None,
        );
        self.object_support
            .notify_transition_end(&transition_end_context)
            .await;

        Ok(())
    }

    async fn execute_triggerless_transitions(
        &self,
        state_context: &dyn StateContext<S, E>,
        state: &dyn StateMachineState<S, E>,
    ) {
        if let Some(state_machine_executor) = self.state_machine_executor.as_ref() {
            state_machine_executor
                .execute_triggerless_transitions(state_context, state)
                .await;
        }
    }
}

#[async_trait]
impl<S, E> Region<S, E> for BaseStateMachine<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn id(&self) -> &str {
        self.id.as_deref().unwrap_or_default()
    }

    async fn start(&self) -> Result<(), BoxError> {
        if self.current_state_id().is_none() && self.initial_enabled != Some(false) {
            let initial_event = self
                .forwarded_initial_event
                .clone()
                .or_else(|| self.initial_event.clone());
            self.apply_transition(self.initial_transition.clone(), initial_event)
                .await?;
        }

        let context = self.build_state_context(
            Stage::StateMachineStart,
            None,
            None,
            self.relay_state_machine(),
            None,
            self.current_state_arc(),
            None,
        );
        self.object_support
            .get_state_listener()
            .state_machine_started(self);
        self.object_support
            .get_state_listener()
            .state_context(&context);

        Ok(())
    }

    async fn stop(&self) -> Result<(), BoxError> {
        let current_state_id = self.current_state_id();
        self.set_last_state_id(current_state_id.clone());
        self.set_current_state_id(None);

        let context = self.build_state_context(
            Stage::StateMachineStop,
            None,
            None,
            self.relay_state_machine(),
            current_state_id.and_then(|id| self.find_state_arc_by_id(&id)),
            None,
            None,
        );
        self.object_support
            .get_state_listener()
            .state_machine_stopped(self);
        self.object_support
            .get_state_listener()
            .state_context(&context);

        Ok(())
    }

    async fn send_event(
        &self,
        mut event: Box<dyn Message<E>>,
    ) -> Result<Box<dyn StateMachineEventResult<S, E>>, BoxError> {
        if self.has_state_machine_error() {
            return Err(state_machine_error("state machine is in error state"));
        }

        self.object_support
            .get_state_machine_interceptors()
            .pre_event(event.as_mut(), self);

        let current_state = self
            .current_state_arc()
            .ok_or_else(|| state_machine_error("state machine has not been started"))?;

        if current_state.should_defer(event.as_ref()) {
            if let Some(state_machine_executor) = self.state_machine_executor.as_ref() {
                state_machine_executor.queue_deferred_event(event.clone());
            }
            let result = current_state.send_event(event).await;
            return Ok(Box::new(result));
        }

        let result = current_state.send_event(event.clone()).await;

        if let Some(transition) = self
            .find_matching_transition(current_state.as_ref(), event.as_ref())
            .await
        {
            self.apply_transition(transition, Some(event.clone()))
                .await?;
        } else if self.parent_machine.is_none() {
            let context = self.build_state_context(
                Stage::EventNotAccepted,
                Some(event.clone()),
                None,
                self.relay_state_machine(),
                self.current_state_arc(),
                None,
                None,
            );
            self.object_support
                .notify_event_not_accepted(&context)
                .await;
        }

        if let Some(current_state) = self.current_state_arc() {
            let context = self.build_state_context(
                Stage::StateChanged,
                Some(event.clone()),
                None,
                self.relay_state_machine(),
                None,
                Some(current_state.clone()),
                None,
            );
            self.execute_triggerless_transitions(&context, current_state.as_ref())
                .await;
        }

        Ok(Box::new(result))
    }

    async fn send_events(
        &self,
        events: Vec<Box<dyn Message<E>>>,
    ) -> Result<Box<dyn StateMachineEventResult<S, E>>, BoxError> {
        let mut last_result: Option<Box<dyn StateMachineEventResult<S, E>>> = None;

        for event in events {
            last_result = Some(self.send_event(event).await?);
        }

        last_result.ok_or_else(|| state_machine_error("send_events requires at least one event"))
    }

    fn state(&self) -> Option<&dyn StateMachineState<S, E>> {
        let current_state_id = self.current_state_id();
        let last_state_id = self.last_state_id();

        if current_state_id.is_none() {
            if let Some(last_state_id) = last_state_id.as_ref() {
                if let Some(state) = self.find_state_ref_by_id(last_state_id) {
                    if matches!(
                        state
                            .pseudo_state()
                            .map(|pseudo_state| pseudo_state.get_kind()),
                        Some(PseudoStateKind::End)
                    ) {
                        return Some(state);
                    }
                }
            }
        }

        current_state_id
            .as_ref()
            .and_then(|state_id| self.find_state_ref_by_id(state_id))
    }

    fn states(&self) -> Vec<&dyn StateMachineState<S, E>> {
        match self.state() {
            Some(state) => vec![state],
            None => Vec::new(),
        }
    }

    fn transitions(&self) -> Vec<&dyn StateMachineTransition<S, E>> {
        self.transitions
            .iter()
            .map(|transition| transition.as_ref())
            .collect()
    }

    fn is_complete(&self) -> bool {
        self.state()
            .and_then(|state| state.pseudo_state())
            .map(|pseudo_state| pseudo_state.get_kind() == PseudoStateKind::End)
            .unwrap_or(false)
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
        listener: &dyn StateMachineListener<S, E>,
    ) -> Result<(), BoxError> {
        self.object_support
            .get_state_listener()
            .unregister(listener);
        Ok(())
    }
}

impl<S, E> StateMachine<S, E> for BaseStateMachine<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Send + Sync + 'static,
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

    fn set_state_machine_error(&mut self, error: Box<dyn std::error::Error + Send>) {
        let intercepted = self
            .object_support
            .get_state_machine_interceptors()
            .state_machine_error(self, Box::new(std::io::Error::other(error.to_string())));
        self.current_error = Some(std::io::Error::other(intercepted.to_string()).into());
    }

    fn has_state_machine_error(&self) -> bool {
        self.current_error.is_some()
    }
}

impl<S, E> StateMachineObjectSupportExt<S, E> for BaseStateMachine<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn notify_event_not_accepted(&self, _state_context: &dyn StateContext<S, E>) {}
}

impl<S, E> LifecycleObjectSupportExt for BaseStateMachine<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn on_init(&mut self) -> Result<(), BoxError> {
        let Some(initial_pseudo_state) = self.initial_state.pseudo_state() else {
            return Err(state_machine_error(
                "initial state must define a pseudo state of kind Initial",
            ));
        };

        if initial_pseudo_state.get_kind() != PseudoStateKind::Initial {
            return Err(state_machine_error(
                "initial state's pseudo state kind must be Initial",
            ));
        }

        self.set_last_state_id(None);
        self.trigger_to_transition.clear();
        self.triggerless_transitions.clear();

        for transition in &self.transitions {
            if let Some(trigger) = transition.trigger() {
                self.trigger_to_transition
                    .push((trigger.id().to_string(), transition.clone()));
            } else {
                self.triggerless_transitions.push(transition.clone());
            }
        }

        self.history = self
            .states
            .iter()
            .find_map(|state| match state.pseudo_state() {
                Some(pseudo_state)
                    if matches!(
                        pseudo_state.get_kind(),
                        PseudoStateKind::HistoryDeep | PseudoStateKind::HistoryShallow
                    ) =>
                {
                    Some(pseudo_state.clone())
                }
                _ => None,
            });

        Ok(())
    }
}

impl<S, E> fmt::Display for BaseStateMachine<S, E>
where
    S: Debug + Clone + Eq + Hash + Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buffer = String::new();

        for state in &self.states {
            for nested_state in state.states() {
                buffer.push_str(&format!("{:?} ", nested_state.id()));
            }
        }

        buffer.push_str(" / ");
        if let Some(current_state) = self.state() {
            buffer.push_str(&format!("{:?}", current_state.ids()));
        }

        buffer.push_str(" / uuid=");
        buffer.push_str(&self.uuid.to_string());
        buffer.push_str(" / id=");
        buffer.push_str(self.id.as_deref().unwrap_or_default());

        write!(f, "{buffer}")
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
