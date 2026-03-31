use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::Arc;

use futures::executor::block_on;
use futures::future::BoxFuture;
use next_web_core::async_trait;
use next_web_core::error::BoxError;
use next_web_core::traits::id::Id;
use next_web_core::traits::message::Message;
use uuid::Uuid;

use crate::config::action::StateMachineAction;
use crate::config::model::state_data::StateData;
use crate::config::model::state_machine_model::StateMachineModel;
use crate::config::model::state_machine_model_factory::StateMachineModelFactory;
use crate::config::model::transition_data::TransitionData;
use crate::config::model::verifier::composite_state_machine_model_verifier::CompositeStateMachineModelVerifier;
use crate::config::model::verifier::state_machine_model_verifier::StateMachineModelVerifier;
use crate::config::state_machine_factory::StateMachineFactory;
use crate::extended_state::ExtendedState;
use crate::monitor::state_machine_monitor::StateMachineMonitor;
use crate::state::StateMachineState;
use crate::state::action_listener::ActionListener;
use crate::state::default_pseudo_state::DefaultPseudoState;
use crate::state::pseudo_state::PseudoState;
use crate::state::pseudo_state_kind::PseudoStateKind;
use crate::state::state_listener::StateListener;
use crate::state_context::StateContext;
use crate::state_machine_event_result::DefaultStateMachineEventResult;
use crate::support::base_state_machine::BaseStateMachine;
use crate::support::default_extended_state::DefaultExtendedState;
use crate::support::lifecycle_object_support::LifecycleObjectSupportExt;
use crate::transition::StateMachineTransition;
use crate::transition::base_transition::BaseTransition;
use crate::transition::initial_transition::InitialTransition;
use crate::transition::transition_kind::TransitionKind;
use crate::trigger::Trigger;
use crate::trigger::trigger_context::TriggerContext;
use crate::trigger::trigger_listener::TriggerListener;
use crate::{BoxedStateAction, StateMachine};

fn factory_error(message: impl Into<String>) -> BoxError {
    std::io::Error::other(message.into()).into()
}

fn unsupported_feature(feature: &str, reason: &str) -> BoxError {
    factory_error(format!(
        "BaseStateMachineFactory does not support {feature}: {reason}"
    ))
}

struct EventTrigger<S, E> {
    event: E,
    _marker: PhantomData<S>,
}

impl<S, E> EventTrigger<S, E> {
    fn new(event: E) -> Self {
        Self {
            event,
            _marker: PhantomData,
        }
    }
}

impl<S, E> Clone for EventTrigger<S, E>
where
    E: Clone,
{
    fn clone(&self) -> Self {
        Self {
            event: self.event.clone(),
            _marker: PhantomData,
        }
    }
}

impl<S, E> Id for EventTrigger<S, E> {
    fn id(&self) -> &str {
        "eventTrigger"
    }
}

#[async_trait]
impl<S, E> Trigger<S, E> for EventTrigger<S, E>
where
    S: Send + Sync + 'static,
    E: Eq + Clone + Send + Sync + 'static,
{
    async fn evaluate(&self, context: &dyn TriggerContext<S, E>) -> bool {
        context.get_event() == &self.event
    }

    async fn add_trigger_listener(&self, _listener: Arc<dyn TriggerListener>) {}

    fn event(&self) -> Option<&E> {
        Some(&self.event)
    }

    async fn arm(&mut self) {}

    async fn disarm(&mut self) {}
}

struct FactoryState<S, E> {
    id: S,
    deferred_events: Vec<E>,
    entry_action_exec: Vec<BoxedStateAction<S, E>>,
    exit_action_exec: Vec<BoxedStateAction<S, E>>,
    state_action_exec: Vec<BoxedStateAction<S, E>>,
    pseudo_state: Option<Arc<dyn PseudoState<S, E>>>,
    submachine: Option<Arc<dyn StateMachine<S, E>>>,
}

impl<S, E> FactoryState<S, E> {
    fn new(
        id: S,
        deferred_events: Vec<E>,
        entry_action_exec: Vec<BoxedStateAction<S, E>>,
        exit_action_exec: Vec<BoxedStateAction<S, E>>,
        state_action_exec: Vec<BoxedStateAction<S, E>>,
        pseudo_state: Option<Arc<dyn PseudoState<S, E>>>,
    ) -> Self {
        Self {
            id,
            deferred_events,
            entry_action_exec,
            exit_action_exec,
            state_action_exec,
            pseudo_state,
            submachine: None,
        }
    }

    fn with_submachine(
        id: S,
        submachine: Arc<dyn StateMachine<S, E>>,
        deferred_events: Vec<E>,
        entry_action_exec: Vec<BoxedStateAction<S, E>>,
        exit_action_exec: Vec<BoxedStateAction<S, E>>,
        pseudo_state: Option<Arc<dyn PseudoState<S, E>>>,
    ) -> Self {
        Self {
            id,
            deferred_events,
            entry_action_exec,
            exit_action_exec,
            state_action_exec: Vec::new(),
            pseudo_state,
            submachine: Some(submachine),
        }
    }
}

#[async_trait]
impl<S, E> StateMachineState<S, E> for FactoryState<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    async fn send_event(&self, event: Box<dyn Message<E>>) -> DefaultStateMachineEventResult<S, E> {
        DefaultStateMachineEventResult::new(
            Some(self.id.clone()),
            event.get_payload().cloned(),
            true,
        )
    }

    fn should_defer(&self, _event: &dyn Message<E>) -> bool {
        false
    }

    async fn exit(&self, context: &dyn StateContext<S, E>) {
        for action in &self.exit_action_exec {
            action(context).await;
        }
    }

    async fn entry(&self, context: &dyn StateContext<S, E>) {
        for action in &self.entry_action_exec {
            action(context).await;
        }

        for action in &self.state_action_exec {
            action(context).await;
        }
    }

    fn id(&self) -> &S {
        &self.id
    }

    fn ids(&self) -> HashSet<&S> {
        let mut ids = HashSet::with_capacity(1);
        ids.insert(&self.id);
        ids
    }

    fn states(&self) -> Vec<&dyn StateMachineState<S, E>> {
        vec![self]
    }

    fn pseudo_state(&self) -> Option<&Arc<dyn PseudoState<S, E>>> {
        self.pseudo_state.as_ref()
    }

    fn deferred_events(&self) -> &[E] {
        &self.deferred_events
    }

    fn entry_actions(&self) -> &[Box<dyn Fn(&dyn StateContext<S, E>) -> BoxFuture<'static, ()>>] {
        &[]
    }

    fn state_actions(&self) -> &[Box<dyn Fn(&dyn StateContext<S, E>) -> BoxFuture<'static, ()>>] {
        &[]
    }

    fn exit_actions(&self) -> &[Box<dyn Fn(&dyn StateContext<S, E>) -> BoxFuture<'static, ()>>] {
        &[]
    }

    fn is_simple(&self) -> bool {
        self.submachine.is_none()
    }

    fn is_composite(&self) -> bool {
        false
    }

    fn is_orthogonal(&self) -> bool {
        false
    }

    fn is_submachine_state(&self) -> bool {
        self.submachine.is_some()
    }

    fn add_state_listener(&mut self, _listener: Arc<dyn StateListener<S, E>>) {}

    fn remove_state_listener(&mut self, _listener: &dyn StateListener<S, E>) {}

    fn add_action_listener(&mut self, _listener: Arc<dyn ActionListener<S, E>>) {}

    fn remove_action_listener(&mut self, _listener: &dyn ActionListener<S, E>) {}
}

/// Base factory translating Spring's machine-building flow into the current Rust API.
#[derive(Clone)]
pub struct BaseStateMachineFactory<S, E> {
    default_state_machine_model: Arc<dyn StateMachineModel<S, E>>,
    state_machine_model_factory: Option<Arc<dyn StateMachineModelFactory<S, E>>>,
    context_events: Option<bool>,
    handle_autostartup: bool,
    name: String,
    default_state_machine_monitor: Option<Arc<dyn StateMachineMonitor<S, E>>>,
}

impl<S, E> BaseStateMachineFactory<S, E>
where
    S: Eq + Hash + Debug,
    S: Clone,
    S: Send + Sync + 'static,
    E: Eq,
    E: Clone,
    E: Send + Sync + 'static,
{
    pub fn new(
        default_state_machine_model: Arc<dyn StateMachineModel<S, E>>,
        state_machine_model_factory: Option<Arc<dyn StateMachineModelFactory<S, E>>>,
    ) -> Self {
        Self {
            default_state_machine_model,
            state_machine_model_factory,
            context_events: None,
            handle_autostartup: false,
            name: String::from("stateMachine"),
            default_state_machine_monitor: None,
        }
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    pub fn set_handle_autostartup(&mut self, handle_autostartup: bool) {
        self.handle_autostartup = handle_autostartup;
    }

    pub fn set_context_events_enabled(&mut self, context_events: bool) {
        self.context_events = Some(context_events);
    }

    pub fn set_state_machine_monitor(
        &mut self,
        state_machine_monitor: Arc<dyn StateMachineMonitor<S, E>>,
    ) {
        self.default_state_machine_monitor = Some(state_machine_monitor);
    }

    fn _get_state_machine(
        &self,
        uuid: Option<Uuid>,
        machine_id: Option<String>,
    ) -> Result<Arc<dyn StateMachine<S, E>>, BoxError> {
        let state_machine_model = self.resolve_state_machine_model(machine_id.as_deref());

        if state_machine_model
            .get_configuration_data()
            .is_verifier_enabled()
        {
            match state_machine_model.get_configuration_data().verifier() {
                Some(verifier) => verifier.verify(state_machine_model.as_ref())?,
                None => CompositeStateMachineModelVerifier::<S, E>::default()
                    .verify(state_machine_model.as_ref())?,
            }
        }

        self.ensure_post_build_features_supported(state_machine_model.as_ref())?;

        let default_extended_state: Arc<dyn ExtendedState> =
            Arc::new(DefaultExtendedState::default());
        let mut state_map: HashMap<S, Arc<dyn StateMachineState<S, E>>> = HashMap::new();
        let mut machine_map: HashMap<S, Arc<dyn StateMachine<S, E>>> = HashMap::new();
        let mut state_stack: Vec<StateData<S, E>> = Vec::new();
        let post_order = self.build_state_data_post_order(state_machine_model.as_ref())?;
        let mut top_level_machine: Option<Arc<dyn StateMachine<S, E>>> = None;

        for state_data in post_order {
            if state_stack.is_empty() {
                state_stack.push(state_data);
                continue;
            }

            let stack_contains_same_parent =
                self.stack_contains_parent_state(&state_stack, state_data.state())?;

            if !stack_contains_same_parent {
                state_stack.push(state_data);
                continue;
            }

            let grouped_states = self.pop_same_parents(&mut state_stack)?;
            let built_machine = self.build_machine_group(
                &machine_map,
                &mut state_map,
                &grouped_states,
                default_extended_state.clone(),
                machine_id.as_deref(),
                uuid,
                state_machine_model.as_ref(),
            )?;

            if let Some(parent_id) = self.group_parent_id(&grouped_states)? {
                machine_map.insert(parent_id, built_machine);
            } else {
                top_level_machine = Some(built_machine);
            }

            state_stack.push(state_data);
        }

        while !state_stack.is_empty() {
            let grouped_states = self.pop_same_parents(&mut state_stack)?;
            let built_machine = self.build_machine_group(
                &machine_map,
                &mut state_map,
                &grouped_states,
                default_extended_state.clone(),
                machine_id.as_deref(),
                uuid,
                state_machine_model.as_ref(),
            )?;

            if let Some(parent_id) = self.group_parent_id(&grouped_states)? {
                machine_map.insert(parent_id, built_machine);
            } else {
                top_level_machine = Some(built_machine);
            }
        }

        match top_level_machine {
            Some(machine) => {
                self.apply_post_build_features(machine.clone(), state_machine_model.as_ref())?;
                Ok(machine)
            }
            None => Err(self.missing_top_level_machine_error(state_machine_model.as_ref())?),
        }
    }

    fn build_state_data_post_order(
        &self,
        state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> Result<Vec<StateData<S, E>>, BoxError> {
        let mut all_states = Vec::new();
        self.collect_all_state_data(
            state_machine_model
                .get_states_data()
                .map(|s| s.state_data())
                .unwrap_or_default(),
            &mut all_states,
        );

        let mut ordered = Vec::new();
        let mut visited = HashSet::new();
        self.collect_post_order(None, &all_states, &mut visited, &mut ordered)?;

        Ok(ordered)
    }

    fn collect_all_state_data(
        &self,
        state_datas: &[StateData<S, E>],
        collected: &mut Vec<StateData<S, E>>,
    ) {
        for state_data in state_datas {
            collected.push(state_data.clone());
            if let Some(submachine_state_data) = state_data.submachine_state_data() {
                self.collect_all_state_data(submachine_state_data, collected);
            }
        }
    }

    fn collect_post_order(
        &self,
        parent: Option<&S>,
        all_states: &[StateData<S, E>],
        visited: &mut HashSet<S>,
        ordered: &mut Vec<StateData<S, E>>,
    ) -> Result<(), BoxError> {
        for state_data in all_states {
            if self.parent_matches(state_data, parent)? {
                if !visited.insert(state_data.state().clone()) {
                    return Err(factory_error(format!(
                        "Detected a cyclic state parent relationship at state {:?}",
                        state_data.state()
                    )));
                }

                self.collect_post_order(Some(state_data.state()), all_states, visited, ordered)?;
                visited.remove(state_data.state());
                ordered.push(state_data.clone());
            }
        }

        Ok(())
    }

    fn resolve_state_machine_model(
        &self,
        machine_id: Option<&str>,
    ) -> Arc<dyn StateMachineModel<S, E>> {
        match self.state_machine_model_factory.as_ref() {
            Some(model_factory) => {
                model_factory.build_with_machine_id(machine_id.unwrap_or_default().to_string())
            }
            None => self.default_state_machine_model.clone(),
        }
    }

    fn ensure_post_build_features_supported(
        &self,
        state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> Result<(), BoxError> {
        let configuration = state_machine_model.get_configuration_data();

        if self.context_events.unwrap_or(false) {
            return Err(unsupported_feature(
                "context events",
                "application context event publication is not implemented",
            ));
        }

        if configuration.is_security_enabled() {
            return Err(unsupported_feature(
                "security interceptors",
                "security post-processing is not implemented",
            ));
        }

        if configuration.ensemble().is_some() {
            return Err(unsupported_feature(
                "distributed state machines",
                "distributed machine wrapping is not implemented",
            ));
        }

        if configuration.state_machine_monitor().is_some()
            || self.default_state_machine_monitor.is_some()
        {
            return Err(unsupported_feature(
                "state machine monitors",
                "monitor attachment is not implemented",
            ));
        }

        if !configuration.interceptors().is_empty() {
            return Err(unsupported_feature(
                "state machine interceptors",
                "interceptor attachment to all regions is not implemented",
            ));
        }

        Ok(())
    }

    fn build_machine_group(
        &self,
        machine_map: &HashMap<S, Arc<dyn StateMachine<S, E>>>,
        state_map: &mut HashMap<S, Arc<dyn StateMachineState<S, E>>>,
        state_datas: &[StateData<S, E>],
        default_extended_state: Arc<dyn ExtendedState>,
        machine_id: Option<&str>,
        uuid: Option<Uuid>,
        state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> Result<Arc<dyn StateMachine<S, E>>, BoxError> {
        let initial_count = self.get_initial_count(state_datas);
        if initial_count > 1 {
            let regions = self.split_into_regions(state_datas);
            return Err(unsupported_feature(
                "orthogonal regions",
                &format!(
                    "found {} initial states across {} region groups, but region-state assembly is not implemented",
                    initial_count,
                    regions.len()
                ),
            ));
        }

        let transitions_data = self.resolve_transition_data(
            state_machine_model
                .get_transitions_data()
                .map(|s| s.transitions())
                .unwrap_or_default(),
            state_datas,
        )?;

        self.build_machine(
            machine_map,
            state_map,
            state_datas,
            &transitions_data,
            default_extended_state,
            machine_id,
            uuid,
            state_machine_model,
        )
    }

    fn resolve_transition_data(
        &self,
        all_transitions: &[TransitionData<S, E>],
        state_datas: &[StateData<S, E>],
    ) -> Result<Vec<TransitionData<S, E>>, BoxError>
    where
        E: Eq,
    {
        let parent_state_id = self.group_parent_id(state_datas)?;
        let mut transitions = Vec::new();

        for transition in all_transitions {
            match (parent_state_id.as_ref(), transition.state()) {
                (Some(parent), Some(state)) if state == parent => {
                    transitions.push(transition.clone())
                }
                (None, None) => transitions.push(transition.clone()),
                _ => {}
            }
        }

        Ok(transitions)
    }

    fn build_machine(
        &self,
        machine_map: &HashMap<S, Arc<dyn StateMachine<S, E>>>,
        state_map: &mut HashMap<S, Arc<dyn StateMachineState<S, E>>>,
        state_datas: &[StateData<S, E>],
        transitions_data: &[TransitionData<S, E>],
        default_extended_state: Arc<dyn ExtendedState>,
        machine_id: Option<&str>,
        uuid: Option<Uuid>,
        state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> Result<Arc<dyn StateMachine<S, E>>, BoxError>
    where
        E: Eq,
    {
        let mut states: Vec<Arc<dyn StateMachineState<S, E>>> = Vec::new();
        let mut initial_state: Option<Arc<dyn StateMachineState<S, E>>> = None;
        let mut initial_actions: Vec<BoxedStateAction<S, E>> = Vec::new();

        for state_data in state_datas {
            if let Some(existing_state) = state_map.get(state_data.state()) {
                let existing_state = existing_state.clone();
                if state_data.is_initial() {
                    initial_state = Some(existing_state.clone());
                }
                states.push(existing_state);
                continue;
            }

            let pseudo_state = self.build_supported_pseudo_state(state_data)?;
            let deferred = state_data
                .deferred()
                .map(|items| items.to_vec())
                .unwrap_or_default();
            let entry_actions = state_data
                .entry_actions()
                .map(|items| items.to_vec())
                .unwrap_or_default();
            let exit_actions = state_data
                .exit_actions()
                .map(|items| items.to_vec())
                .unwrap_or_default();
            let state_actions = state_data
                .state_actions()
                .map(|items| items.to_vec())
                .unwrap_or_default();

            let state = if let Some(submachine) = machine_map.get(state_data.state()) {
                self.build_submachine_state_internal(
                    state_data.state().clone(),
                    submachine.clone(),
                    deferred,
                    entry_actions,
                    exit_actions,
                    pseudo_state,
                )?
            } else if let Some(submachine) = state_data.submachine_arc() {
                self.build_submachine_state_internal(
                    state_data.state().clone(),
                    submachine,
                    deferred,
                    entry_actions,
                    exit_actions,
                    pseudo_state,
                )?
            } else if let Some(submachine_factory) = state_data.submachine_factory_arc() {
                let resolved_machine_id = machine_id
                    .map(str::to_owned)
                    .unwrap_or_else(|| self.resolve_machine_id(state_machine_model, None));
                let submachine =
                    submachine_factory.get_state_machine_with_id(resolved_machine_id)?;
                self.build_submachine_state_internal(
                    state_data.state().clone(),
                    submachine,
                    deferred,
                    entry_actions,
                    exit_actions,
                    pseudo_state,
                )?
            } else {
                self.build_state_internal(
                    state_data.state().clone(),
                    deferred,
                    entry_actions,
                    exit_actions,
                    state_actions,
                    pseudo_state,
                    state_machine_model,
                )?
            };

            if state_data.is_initial() {
                initial_state = Some(state.clone());
                initial_actions = state_data
                    .initial_action_arc()
                    .map(|action| vec![self.wrap_state_machine_action(action)])
                    .unwrap_or_default();
            }

            state_map.insert(state_data.state().clone(), state.clone());
            states.push(state);
        }

        if initial_state.is_none() && states.len() == 1 {
            initial_state = states.first().cloned();
        }

        let initial_state = initial_state
            .ok_or_else(|| factory_error(" not resolve an initial state for the current group"))?;

        let mut transitions: Vec<Arc<dyn StateMachineTransition<S, E>>> = Vec::new();
        for transition_data in transitions_data {
            if transition_data.period().is_some() || transition_data.count().is_some() {
                return Err(unsupported_feature(
                    "timer or counted transitions",
                    "trigger implementations for timed transitions are not available in the current factory path",
                ));
            }

            if transition_data.has_guard() {
                return Err(unsupported_feature(
                    "transition guards",
                    "TransitionData guard function types are not yet compatible with runtime boxed state guards",
                ));
            }

            let trigger = self.build_transition_trigger(transition_data)?;
            let security_rule = transition_data.security_rule().cloned();
            let name = if transition_data.name().is_empty() {
                None
            } else {
                Some(transition_data.name().to_string())
            };

            match transition_data.kind() {
                TransitionKind::Local => {
                    let Some(source) = state_map.get(transition_data.source()).cloned() else {
                        continue;
                    };
                    let Some(target) = state_map.get(transition_data.target()).cloned() else {
                        continue;
                    };

                    let transition = BaseTransition::with_security_and_name(
                        Some(source),
                        target,
                        transition_data.cloned_actions(),
                        transition_data.event().cloned(),
                        TransitionKind::Local,
                        None,
                        trigger,
                        security_rule,
                        name,
                    );
                    transitions.push(Arc::new(transition));
                }
                TransitionKind::External => {
                    let Some(source) = state_map.get(transition_data.source()).cloned() else {
                        continue;
                    };
                    let Some(target) = state_map.get(transition_data.target()).cloned() else {
                        continue;
                    };

                    let transition = BaseTransition::with_security_and_name(
                        Some(source),
                        target,
                        transition_data.cloned_actions(),
                        transition_data.event().cloned(),
                        TransitionKind::External,
                        None,
                        trigger,
                        security_rule,
                        name,
                    );
                    transitions.push(Arc::new(transition));
                }
                TransitionKind::Internal => {
                    let source = state_map
                        .get(transition_data.source())
                        .cloned()
                        .ok_or_else(|| {
                            factory_error(format!(
                                "Internal transition source state {:?} was not built",
                                transition_data.source()
                            ))
                        })?;

                    let transition = BaseTransition::with_security_and_name(
                        Some(source.clone()),
                        source,
                        transition_data.cloned_actions(),
                        transition_data.event().cloned(),
                        TransitionKind::Internal,
                        None,
                        trigger,
                        security_rule,
                        name,
                    );
                    transitions.push(Arc::new(transition));
                }
                TransitionKind::Initial => {
                    return Err(unsupported_feature(
                        "model-defined initial transitions",
                        "initial transitions are derived from state metadata, not TransitionData, in the current factory path",
                    ));
                }
            }
        }

        let initial_transition: Arc<dyn StateMachineTransition<S, E>> =
            if initial_actions.is_empty() {
                Arc::new(InitialTransition::new(initial_state.clone()))
            } else {
                Arc::new(InitialTransition::with_actions(
                    initial_state.clone(),
                    initial_actions,
                ))
            };

        self.build_state_machine_internal(
            states,
            transitions,
            initial_state,
            initial_transition,
            default_extended_state,
            self.resolve_machine_id(state_machine_model, machine_id),
            uuid,
            state_machine_model,
        )
    }

    fn build_supported_pseudo_state(
        &self,
        state_data: &StateData<S, E>,
    ) -> Result<Option<Arc<dyn PseudoState<S, E>>>, BoxError> {
        let kind = if state_data.is_initial() {
            Some(PseudoStateKind::Initial)
        } else if state_data.is_end() {
            Some(PseudoStateKind::End)
        } else {
            state_data.pseudo_state_kind()
        };

        match kind {
            Some(PseudoStateKind::Initial) => Ok(Some(Arc::new(DefaultPseudoState::new(
                PseudoStateKind::Initial,
            )))),
            Some(PseudoStateKind::End) => Ok(Some(Arc::new(DefaultPseudoState::new(
                PseudoStateKind::End,
            )))),
            Some(PseudoStateKind::HistoryShallow) => Err(unsupported_feature(
                "history shallow pseudo states",
                "HistoryPseudoState support is not implemented",
            )),
            Some(PseudoStateKind::HistoryDeep) => Err(unsupported_feature(
                "history deep pseudo states",
                "HistoryPseudoState support is not implemented",
            )),
            Some(PseudoStateKind::Choice) => Err(unsupported_feature(
                "choice pseudo states",
                "ChoicePseudoState support is not implemented",
            )),
            Some(PseudoStateKind::Junction) => Err(unsupported_feature(
                "junction pseudo states",
                "JunctionPseudoState support is not implemented",
            )),
            Some(PseudoStateKind::Fork) => Err(unsupported_feature(
                "fork pseudo states",
                "ForkPseudoState support is not implemented",
            )),
            Some(PseudoStateKind::Join) => Err(unsupported_feature(
                "join pseudo states",
                "JoinPseudoState support is not implemented",
            )),
            Some(PseudoStateKind::Entry) => Err(unsupported_feature(
                "entry pseudo states",
                "EntryPseudoState support is not implemented",
            )),
            Some(PseudoStateKind::Exit) => Err(unsupported_feature(
                "exit pseudo states",
                "ExitPseudoState support is not implemented",
            )),
            None => Ok(None),
        }
    }

    fn build_state_machine_internal(
        &self,
        states: Vec<Arc<dyn StateMachineState<S, E>>>,
        transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
        initial_state: Arc<dyn StateMachineState<S, E>>,
        initial_transition: Arc<dyn StateMachineTransition<S, E>>,
        extended_state: Arc<dyn ExtendedState>,
        machine_id: String,
        uuid: Option<Uuid>,
        _state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> Result<Arc<dyn StateMachine<S, E>>, BoxError> {
        let mut machine = BaseStateMachine::with_all(
            states,
            transitions,
            initial_state,
            Some(initial_transition),
            None,
            extended_state,
            uuid,
        );
        machine.on_init()?;
        machine.set_id(machine_id);
        Ok(Arc::new(machine))
    }

    fn apply_post_build_features(
        &self,
        machine: Arc<dyn StateMachine<S, E>>,
        state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> Result<(), BoxError> {
        let configuration = state_machine_model.get_configuration_data();

        if self.handle_autostartup || configuration.is_auto_start() {
            block_on(machine.start())?;
        }

        for listener in &configuration.listeners {
            machine.add_state_listener(listener.clone())?;
        }

        Ok(())
    }

    fn build_state_internal(
        &self,
        id: S,
        deferred: Vec<E>,
        entry_actions: Vec<BoxedStateAction<S, E>>,
        exit_actions: Vec<BoxedStateAction<S, E>>,
        state_actions: Vec<BoxedStateAction<S, E>>,
        pseudo_state: Option<Arc<dyn PseudoState<S, E>>>,
        _state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> Result<Arc<dyn StateMachineState<S, E>>, BoxError> {
        Ok(Arc::new(FactoryState::new(
            id,
            deferred,
            entry_actions,
            exit_actions,
            state_actions,
            pseudo_state,
        )))
    }

    fn build_submachine_state_internal(
        &self,
        id: S,
        submachine: Arc<dyn StateMachine<S, E>>,
        deferred: Vec<E>,
        entry_actions: Vec<BoxedStateAction<S, E>>,
        exit_actions: Vec<BoxedStateAction<S, E>>,
        pseudo_state: Option<Arc<dyn PseudoState<S, E>>>,
    ) -> Result<Arc<dyn StateMachineState<S, E>>, BoxError> {
        Ok(Arc::new(FactoryState::with_submachine(
            id,
            submachine,
            deferred,
            entry_actions,
            exit_actions,
            pseudo_state,
        )))
    }

    fn build_region_state_internal(
        &self,
        _id: S,
        _regions: Vec<Arc<dyn StateMachine<S, E>>>,
        _deferred: Vec<E>,
        _entry_actions: Vec<BoxedStateAction<S, E>>,
        _exit_actions: Vec<BoxedStateAction<S, E>>,
        _pseudo_state: Option<Arc<dyn PseudoState<S, E>>>,
        _state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> Result<Arc<dyn StateMachineState<S, E>>, BoxError> {
        Err(unsupported_feature(
            "region states",
            "RegionState wiring is not implemented in the current Rust crate",
        ))
    }

    fn resolve_machine_id(
        &self,
        state_machine_model: &dyn StateMachineModel<S, E>,
        machine_id: Option<&str>,
    ) -> String {
        machine_id
            .map(str::to_owned)
            .or_else(|| {
                state_machine_model
                    .get_configuration_data()
                    .machine_id()
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| self.name.clone())
    }

    fn parent_state_id(&self, state_data: &StateData<S, E>) -> Result<Option<S>, BoxError> {
        match state_data.parent() {
            Some(parent) => parent
                .downcast_ref::<S>()
                .cloned()
                .map(Some)
                .ok_or_else(|| {
                    unsupported_feature(
                        "non-state parent identifiers",
                        &format!(
                            "state {:?} has a parent that cannot be downcast into the factory state type",
                            state_data.state()
                        ),
                    )
                }),
            None => Ok(None),
        }
    }

    fn missing_top_level_machine_error(
        &self,
        state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> Result<BoxError, BoxError> {
        let states = state_machine_model
            .get_states_data()
            .map(|states| states.state_data())
            .unwrap_or(&[]);

        if states.is_empty() {
            return Ok(factory_error(
                "BaseStateMachineFactory could not build a top-level state machine because the model contains no states",
            ));
        }

        let mut root_states = Vec::new();
        let mut summary = Vec::with_capacity(states.len());

        for state_data in states {
            match self.parent_state_id(state_data)? {
                Some(parent) => summary.push(format!(
                    "state={:?}, parent={:?}, initial={}",
                    state_data.state(),
                    parent,
                    state_data.is_initial()
                )),
                None => {
                    root_states.push(format!("{:?}", state_data.state()));
                    summary.push(format!(
                        "state={:?}, parent=<root>, initial={}",
                        state_data.state(),
                        state_data.is_initial()
                    ));
                }
            }
        }

        if root_states.is_empty() {
            Ok(factory_error(format!(
                "BaseStateMachineFactory could not build a top-level state machine because no root states were found. Every configured state has a parent. States: {}",
                summary.join("; ")
            )))
        } else {
            Ok(factory_error(format!(
                "BaseStateMachineFactory could not build a top-level state machine even though root states exist: {}. States: {}",
                root_states.join(", "),
                summary.join("; ")
            )))
        }
    }

    fn parent_matches(
        &self,
        state_data: &StateData<S, E>,
        parent: Option<&S>,
    ) -> Result<bool, BoxError> {
        Ok(self.parent_state_id(state_data)?.as_ref() == parent)
    }

    fn wrap_state_machine_action(
        &self,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> BoxedStateAction<S, E> {
        Arc::new(move |context| -> BoxFuture<'_, ()> {
            let action = action.clone();
            Box::pin(async move {
                let _ = action.execute(context).await;
            })
        })
    }

    fn build_transition_trigger(
        &self,
        transition_data: &TransitionData<S, E>,
    ) -> Result<Option<Arc<dyn Trigger<S, E>>>, BoxError> {
        if let Some(event) = transition_data.event().cloned() {
            return Ok(Some(Arc::new(EventTrigger::new(event))));
        }

        if transition_data.period().is_some() || transition_data.count().is_some() {
            return Err(unsupported_feature(
                "timer or counted transitions",
                "timer trigger orchestration is not yet wired into BaseStateMachine execution",
            ));
        }

        Ok(None)
    }

    fn stack_contains_parent_state(
        &self,
        stack: &[StateData<S, E>],
        state: &S,
    ) -> Result<bool, BoxError> {
        for item in stack {
            if self.parent_state_id(item)?.as_ref() == Some(state) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn pop_same_parents(
        &self,
        stack: &mut Vec<StateData<S, E>>,
    ) -> Result<Vec<StateData<S, E>>, BoxError> {
        let parent = match stack.last() {
            Some(item) => self.parent_state_id(item)?,
            None => return Ok(Vec::new()),
        };

        let mut grouped = Vec::new();
        while let Some(item) = stack.last() {
            if self.parent_state_id(item)? == parent {
                grouped.push(stack.pop().expect("stack element exists"));
            } else {
                break;
            }
        }
        grouped.reverse();
        Ok(grouped)
    }

    fn group_parent_id(&self, state_datas: &[StateData<S, E>]) -> Result<Option<S>, BoxError> {
        match state_datas.first() {
            Some(first) => self.parent_state_id(first),
            None => Ok(None),
        }
    }

    fn get_initial_count(&self, state_datas: &[StateData<S, E>]) -> usize {
        state_datas
            .iter()
            .filter(|state_data| state_data.is_initial())
            .count()
    }

    fn split_into_regions(&self, state_datas: &[StateData<S, E>]) -> Vec<Vec<StateData<S, E>>> {
        let mut grouped: HashMap<Option<String>, Vec<StateData<S, E>>> = HashMap::new();
        for state_data in state_datas {
            grouped
                .entry(state_data.region().map(str::to_owned))
                .or_default()
                .push(state_data.clone());
        }
        grouped.into_values().collect()
    }
}

impl<S, E> StateMachineFactory<S, E> for BaseStateMachineFactory<S, E>
where
    S: Clone + Eq + Hash + Debug + Send + Sync + 'static,
    E: Clone + Eq + Send + Sync + 'static,
{
    fn get_state_machine(&self) -> Result<Arc<dyn StateMachine<S, E>>, BoxError> {
        self._get_state_machine(None, None)
    }

    fn get_state_machine_with_id(
        &self,
        machine_id: String,
    ) -> Result<Arc<dyn StateMachine<S, E>>, BoxError> {
        self._get_state_machine(None, Some(machine_id))
    }

    fn get_state_machine_with_uuid(
        &self,
        machine_id: Uuid,
    ) -> Result<Arc<dyn StateMachine<S, E>>, BoxError> {
        self._get_state_machine(Some(machine_id), None)
    }
}

#[cfg(test)]
mod tests {
    use std::{any::Any, sync::Arc};

    use futures::executor::block_on;
    use next_web_core::messaging::generic_message::GenericMessage;

    use crate::config::model::{
        configuration_data::ConfigurationData,
        default_state_machine_model::DefaultStateMachineModel, state_data::StateData,
        transition_data::TransitionData, transitions_data::TransitionsData,
    };
    use crate::config::state_machine_factory::StateMachineFactory;

    use super::BaseStateMachineFactory;

    fn factory() -> BaseStateMachineFactory<i32, i32> {
        let model = DefaultStateMachineModel::new(
            ConfigurationData::default(),
            Some(Default::default()),
            Some(Default::default()),
        );

        BaseStateMachineFactory::new(Arc::new(model), None)
    }

    #[test]
    fn post_order_respects_parent_hierarchy() {
        let factory = factory();

        let top = StateData::new(1);
        let child = StateData::with_hierarchy(Some(Arc::new(1) as Arc<dyn Any>), None, 2, false);
        let sibling = StateData::new(3);

        let model = DefaultStateMachineModel::new(
            ConfigurationData::default(),
            Some(vec![top, child, sibling].into()),
            Some(Default::default()),
        );

        let ordered = factory.build_state_data_post_order(&model).unwrap();
        let ids = ordered
            .into_iter()
            .map(|state| *state.state())
            .collect::<Vec<_>>();

        assert_eq!(ids, vec![2, 1, 3]);
    }

    #[test]
    fn nested_group_uses_parent_scoped_transitions() {
        let factory = factory();

        let child_a = StateData::with_hierarchy(Some(Arc::new(1) as Arc<dyn Any>), None, 10, true);
        let child_b = StateData::with_hierarchy(Some(Arc::new(1) as Arc<dyn Any>), None, 11, false);
        let group = vec![child_a, child_b];

        let by_parent = TransitionData::with_all(
            10,
            11,
            Some(1),
            Some(100),
            None,
            None,
            Vec::new(),
            None,
            crate::transition::transition_kind::TransitionKind::External,
            None,
            "",
        );
        let by_child = TransitionData::with_all(
            10,
            11,
            Some(10),
            Some(101),
            None,
            None,
            Vec::new(),
            None,
            crate::transition::transition_kind::TransitionKind::External,
            None,
            "",
        );
        let top_level = TransitionData::with_all(
            1,
            10,
            None,
            Some(102),
            None,
            None,
            Vec::new(),
            None,
            crate::transition::transition_kind::TransitionKind::External,
            None,
            "",
        );

        let resolved = factory
            .resolve_transition_data(&[by_parent, by_child, top_level], &group)
            .unwrap();

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].state(), Some(&1));
    }

    #[test]
    fn top_level_group_uses_only_root_transitions() {
        let factory = factory();

        let top_a = StateData::with_initial(1, true);
        let top_b = StateData::new(2);
        let group = vec![top_a, top_b];

        let nested = TransitionData::with_all(
            1,
            2,
            Some(1),
            Some(100),
            None,
            None,
            Vec::new(),
            None,
            crate::transition::transition_kind::TransitionKind::External,
            None,
            "",
        );
        let root = TransitionData::with_all(
            1,
            2,
            None,
            Some(101),
            None,
            None,
            Vec::new(),
            None,
            crate::transition::transition_kind::TransitionKind::External,
            None,
            "",
        );

        let resolved = factory
            .resolve_transition_data(&[nested, root], &group)
            .unwrap();

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].state(), None);
    }

    #[test]
    fn missing_top_level_machine_reports_absent_root_states() {
        let mut configuration = ConfigurationData::default();
        configuration.verifier_enabled = false;

        let model = DefaultStateMachineModel::new(
            configuration,
            Some(
                vec![StateData::with_hierarchy(
                    Some(Arc::new(1) as Arc<dyn Any>),
                    None,
                    2,
                    true,
                )]
                .into(),
            ),
            Some(TransitionsData::new(vec![TransitionData::new(2, 2, 100)])),
        );

        let factory = BaseStateMachineFactory::new(Arc::new(model), None);
        let error = match factory.get_state_machine() {
            Ok(_) => panic!("expected factory construction to fail without root states"),
            Err(error) => error.to_string(),
        };

        assert!(error.contains("no root states were found"));
        assert!(error.contains("state=2"));
        assert!(error.contains("parent=1"));
    }

    #[test]
    fn transition_actions_do_not_block_factory_build() {
        let mut configuration = ConfigurationData::default();
        configuration.verifier_enabled = false;

        let model = DefaultStateMachineModel::new(
            configuration,
            Some(vec![StateData::with_initial(1, true), StateData::new(2)].into()),
            Some(TransitionsData::new(vec![TransitionData::with_all(
                1,
                2,
                None,
                Some(100),
                None,
                None,
                vec![Arc::new(|_context| Box::pin(async move {}))],
                None,
                crate::transition::transition_kind::TransitionKind::External,
                None,
                "",
            )])),
        );

        let factory = BaseStateMachineFactory::new(Arc::new(model), None);
        assert!(factory.get_state_machine().is_ok());
    }

    #[test]
    fn built_machine_can_process_event() {
        let mut configuration = ConfigurationData::default();
        configuration.verifier_enabled = false;

        let model = DefaultStateMachineModel::new(
            configuration,
            Some(vec![StateData::with_initial(1, true), StateData::new(2)].into()),
            Some(TransitionsData::new(vec![TransitionData::new(1, 2, 100)])),
        );

        let factory = BaseStateMachineFactory::new(Arc::new(model), None);
        let machine = factory.get_state_machine().unwrap();

        block_on(machine.start()).unwrap();
        block_on(machine.send_event(Box::new(GenericMessage::with_payload(100)))).unwrap();

        assert_eq!(machine.state().map(|state| *state.id()), Some(2));
    }
}
