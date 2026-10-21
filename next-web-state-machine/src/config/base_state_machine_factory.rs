use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;
use std::time::Duration;

use futures::future::BoxFuture;
use next_web_core::error::BoxError;
use uuid::Uuid;

use crate::BoxedStateGuard;
use crate::config::action::StateMachineAction;
use crate::config::guard::StateMachineGuard;
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
use crate::state::choice_pseudo_state::{ChoicePseudoState, ChoiceStateData};
use crate::state::default_pseudo_state::DefaultPseudoState;
use crate::state::entry_pseudo_state::EntryPseudoState;
use crate::state::exit_pseudo_state::ExitPseudoState;
use crate::state::fork_pseudo_state::ForkPseudoState;
use crate::state::history_pseudo_state::HistoryPseudoState;
use crate::state::join_pseudo_state::{JoinPseudoState, JoinStateData};
use crate::state::junction_pseudo_state::{JunctionPseudoState, JunctionStateData};
use crate::state::pseudo_state::PseudoState;
use crate::state::pseudo_state_kind::PseudoStateKind;
use crate::state::state_holder::StateHolder;
use crate::support::base_state_machine::BaseStateMachine;
use crate::support::default_extended_state::DefaultExtendedState;
use crate::transition::StateMachineTransition;
use crate::transition::base_transition::BaseTransition;
use crate::transition::initial_transition::InitialTransition;
use crate::transition::transition_kind::TransitionKind;
use crate::trigger::Trigger;
use crate::{BoxedStateAction, StateMachine};

fn factory_error(message: impl Into<String>) -> BoxError {
    std::io::Error::other(message.into()).into()
}

fn unsupported_feature(feature: &str, reason: &str) -> BoxError {
    factory_error(format!(
        "BaseStateMachineFactory does not support {feature}: {reason}"
    ))
}

fn event_trigger<S, E>(event: E) -> Arc<dyn Trigger<S, E>>
where
    S: Send + Sync + 'static,
    E: Clone + PartialEq + Send + Sync + 'static,
{
    Arc::new(crate::trigger::event_trigger::EventTrigger::<S, E>::new(event))
}

/// Base factory translating Next machine-building flow into the current Rust API.
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
    E: Eq + Hash,
    E: Clone,
    E: Send + Sync + 'static,
{
    /// Creates a new factory.
    ///
    /// # Arguments
    /// * `default_state_machine_model` - the model used when no model factory is set
    /// * `state_machine_model_factory` - the factory which builds the model by machine id
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

    /// Sets the name of the machines this factory builds.
    ///
    /// # Arguments
    /// * `name` - the machine name
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Sets whether the factory starts a machine it builds.
    ///
    /// # Arguments
    /// * `handle_autostartup` - whether to start the machine
    pub fn set_handle_autostartup(&mut self, handle_autostartup: bool) {
        self.handle_autostartup = handle_autostartup;
    }

    /// Sets whether events from the extended state are forwarded to the machine.
    ///
    /// # Arguments
    /// * `context_events` - whether context events are enabled
    pub fn set_context_events_enabled(&mut self, context_events: bool) {
        self.context_events = Some(context_events);
    }

    /// Sets the monitor the factory attaches to the machines it builds.
    ///
    /// # Arguments
    /// * `state_machine_monitor` - the monitor
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
        let mut region_map: HashMap<S, Vec<Arc<dyn crate::region::Region<S, E>>>> = HashMap::new();
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
                &mut region_map,
                &grouped_states,
                default_extended_state.clone(),
                machine_id.as_deref(),
                uuid,
                state_machine_model.as_ref(),
            )?;

            if let Some(parent_id) = self.group_parent_id(&grouped_states)? {
                if let Some(built_machine) = built_machine {
                    machine_map.insert(parent_id, built_machine);
                }
            } else {
                top_level_machine = built_machine;
            }

            state_stack.push(state_data);
        }

        while !state_stack.is_empty() {
            let grouped_states = self.pop_same_parents(&mut state_stack)?;
            let built_machine = self.build_machine_group(
                &machine_map,
                &mut state_map,
                &mut region_map,
                &grouped_states,
                default_extended_state.clone(),
                machine_id.as_deref(),
                uuid,
                state_machine_model.as_ref(),
            )?;

            if let Some(parent_id) = self.group_parent_id(&grouped_states)? {
                if let Some(built_machine) = built_machine {
                    machine_map.insert(parent_id, built_machine);
                }
            } else {
                top_level_machine = built_machine;
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
                model_factory.build_with_machine_id(machine_id.unwrap_or_default().to_owned())
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

        if configuration.ensemble().is_some() {
            return Err(unsupported_feature(
                "distributed state machines",
                "distributed machine wrapping is not implemented",
            ));
        }

        Ok(())
    }

    fn build_machine_group(
        &self,
        machine_map: &HashMap<S, Arc<dyn StateMachine<S, E>>>,
        state_map: &mut HashMap<S, Arc<dyn StateMachineState<S, E>>>,
        region_map: &mut HashMap<S, Vec<Arc<dyn crate::region::Region<S, E>>>>,
        state_datas: &[StateData<S, E>],
        default_extended_state: Arc<dyn ExtendedState>,
        machine_id: Option<&str>,
        uuid: Option<Uuid>,
        state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> Result<Option<Arc<dyn StateMachine<S, E>>>, BoxError> {
        let initial_count = self.get_initial_count(state_datas);
        if initial_count > 1 {
            let regions = self.split_into_regions(state_datas);

            // Orthogonal assembly groups the children of one parent by their
            // region key, so the key has to be set for every region.
            if regions.len() <= 1 {
                return Err(factory_error(
                    "the group has several initial states but no distinct region keys; set StateData::region for every region",
                ));
            }

            let Some(parent_id) = self.group_parent_id(state_datas)? else {
                return Err(factory_error(
                    "an orthogonal group needs a parent state which owns its regions",
                ));
            };

            let mut machines: Vec<Arc<dyn crate::region::Region<S, E>>> = Vec::new();
            for region_states in regions {
                let built = self.build_machine_group(
                    machine_map,
                    state_map,
                    region_map,
                    &region_states,
                    default_extended_state.clone(),
                    machine_id,
                    uuid,
                    state_machine_model,
                )?;

                if let Some(machine) = built {
                    // The features are attached by `build_machine`, which every
                    // machine of the group passes through.
                    machines.push(machine);
                }
            }

            // The parent state is built by the group of its own parent, which
            // picks the regions up from this map.
            region_map.insert(parent_id, machines);
            return Ok(None);
        }

        let transitions_data = self.resolve_transition_data(
            state_machine_model
                .get_transitions_data()
                .map(|s| s.transitions())
                .unwrap_or_default(),
            state_datas,
        )?;

        let machine = self.build_machine(
            machine_map,
            state_map,
            region_map,
            state_datas,
            &transitions_data,
            default_extended_state,
            machine_id,
            uuid,
            state_machine_model,
        )?;

        Ok(Some(machine))
    }

    fn resolve_transition_data(
        &self,
        all_transitions: &[TransitionData<S, E>],
        state_datas: &[StateData<S, E>],
    ) -> Result<Vec<TransitionData<S, E>>, BoxError>
    where
        E: Eq + Hash,
    {
        let parent_state_id = self.group_parent_id(state_datas)?;
        let belongs_to_group = |state: &S| state_datas.iter().any(|data| data.state() == state);
        let mut transitions = Vec::new();

        for transition in all_transitions {
            // A scoped transition belongs to the group of the state which scopes
            // it, which is what a region uses to reach into its own states. An
            // unscoped transition belongs to the group which owns its endpoints,
            // and the top level group is the fallback for the ones which cross a
            // boundary.
            let belongs = match (parent_state_id.as_ref(), transition.state()) {
                (Some(parent), Some(scope)) => scope == parent,
                (None, Some(_)) => false,
                (Some(_), None) => {
                    belongs_to_group(transition.source()) && belongs_to_group(transition.target())
                }
                (None, None) => {
                    belongs_to_group(transition.source()) || belongs_to_group(transition.target())
                }
            };

            if belongs {
                transitions.push(transition.clone());
            }
        }

        Ok(transitions)
    }

    fn build_machine(
        &self,
        machine_map: &HashMap<S, Arc<dyn StateMachine<S, E>>>,
        state_map: &mut HashMap<S, Arc<dyn StateMachineState<S, E>>>,
        region_map: &HashMap<S, Vec<Arc<dyn crate::region::Region<S, E>>>>,
        state_datas: &[StateData<S, E>],
        transitions_data: &[TransitionData<S, E>],
        default_extended_state: Arc<dyn ExtendedState>,
        machine_id: Option<&str>,
        uuid: Option<Uuid>,
        state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> Result<Arc<dyn StateMachine<S, E>>, BoxError>
    where
        E: Eq + Hash,
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

            // Pseudo states are attached in a later pass, once every target
            // state of this group exists.
            let pseudo_state: Option<Arc<dyn PseudoState<S, E>>> = None;
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

            let state = if let Some(regions) = region_map.get(state_data.state()) {
                self.build_region_state_internal(
                    state_data.state().clone(),
                    regions.clone(),
                    deferred,
                    entry_actions,
                    exit_actions,
                    pseudo_state,
                    state_machine_model,
                )?
            } else if let Some(submachine) = machine_map.get(state_data.state()) {
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
            let trigger = self.build_transition_trigger(transition_data)?;
            let security_rule = transition_data.security_rule().cloned();
            let name = if transition_data.name().is_empty() {
                None
            } else {
                Some(transition_data.name().to_owned())
            };

            match transition_data.kind() {
                TransitionKind::Local => {
                    let Some(source) = state_map.get(transition_data.source()).cloned() else {
                        continue;
                    };
                    let Some(target) = state_map.get(transition_data.target()).cloned() else {
                        continue;
                    };

                    let transition = BaseTransition::new(
                        Some(source),
                        target,
                        transition_data.cloned_actions(),
                        TransitionKind::Local,
                        transition_data.cloned_guard(),
                        trigger,
                    )
                    .with_security_rule(security_rule)
                    .with_name(name.unwrap_or_default());
                    transitions.push(Arc::new(transition));
                }
                TransitionKind::External => {
                    let Some(source) = state_map.get(transition_data.source()).cloned() else {
                        continue;
                    };
                    let Some(target) = state_map.get(transition_data.target()).cloned() else {
                        continue;
                    };

                    let transition = BaseTransition::new(
                        Some(source),
                        target,
                        transition_data.cloned_actions(),
                        TransitionKind::External,
                        transition_data.cloned_guard(),
                        trigger,
                    )
                    .with_security_rule(security_rule)
                    .with_name(name.unwrap_or_default());
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

                    let transition = BaseTransition::new(
                        Some(source.clone()),
                        source,
                        transition_data.cloned_actions(),
                        TransitionKind::Internal,
                        transition_data.cloned_guard(),
                        trigger,
                    )
                    .with_security_rule(security_rule)
                    .with_name(name.unwrap_or_default());
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

        self.apply_pseudo_states(&states, state_map, state_datas, state_machine_model)?;

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

    /// Attaches the pseudo states of a built group.
    ///
    /// The states of a group are built before its transitions, and a pseudo
    /// state refers to the states it continues with, so the pseudo states are
    /// attached in a second pass, when every state of the group exists.
    ///
    /// # Arguments
    /// * `states` - the states built for this group
    /// * `state_map` - all states built so far, by identifier
    /// * `state_datas` - the model data of this group
    /// * `state_machine_model` - the model which holds the pseudo state data
    fn apply_pseudo_states(
        &self,
        states: &[Arc<dyn StateMachineState<S, E>>],
        state_map: &HashMap<S, Arc<dyn StateMachineState<S, E>>>,
        state_datas: &[StateData<S, E>],
        state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> Result<(), BoxError>
    where
        E: Hash,
    {
        let transitions_data = state_machine_model.get_transitions_data();

        for state_data in state_datas {
            let id = state_data.state().clone();
            let Some(state) = states.iter().find(|state| state.id() == &id).cloned() else {
                continue;
            };

            let kind = if state_data.is_initial() {
                Some(PseudoStateKind::Initial)
            } else if state_data.is_end() {
                Some(PseudoStateKind::End)
            } else {
                state_data.pseudo_state_kind()
            };

            let Some(kind) = kind else {
                continue;
            };

            let pseudo: Arc<dyn PseudoState<S, E>> = match kind {
                PseudoStateKind::Initial | PseudoStateKind::End => {
                    Arc::new(DefaultPseudoState::new(kind))
                }
                PseudoStateKind::Choice => {
                    let entries = transitions_data
                        .and_then(|data| data.choices())
                        .and_then(|choices| choices.get(&id))
                        .map(|choices| {
                            choices
                                .iter()
                                .filter_map(|choice| {
                                    let target = state_map.get(choice.target())?.clone();
                                    Some(ChoiceStateData::new(
                                        target,
                                        Some(self.guard_adapter(choice.guard_arc())),
                                        choice
                                            .actions()
                                            .iter()
                                            .map(|action| {
                                                self.wrap_state_machine_action(action.clone())
                                            })
                                            .collect(),
                                    ))
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();

                    Arc::new(ChoicePseudoState::new(entries))
                }
                PseudoStateKind::Junction => {
                    let entries = transitions_data
                        .and_then(|data| data.junctions())
                        .and_then(|junctions| junctions.get(&id))
                        .map(|junctions| {
                            junctions
                                .iter()
                                .filter_map(|junction| {
                                    let target = state_map.get(junction.target())?.clone();
                                    Some(JunctionStateData::new(
                                        target,
                                        Some(self.guard_adapter(junction.guard_arc())),
                                        junction
                                            .actions()
                                            .iter()
                                            .map(|action| {
                                                self.wrap_state_machine_action(action.clone())
                                            })
                                            .collect(),
                                    ))
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();

                    Arc::new(JunctionPseudoState::new(entries))
                }
                PseudoStateKind::Fork => {
                    let forks = transitions_data
                        .and_then(|data| data.forks())
                        .and_then(|forks| forks.get(&id))
                        .map(|targets| {
                            targets
                                .iter()
                                .filter_map(|target| state_map.get(target).cloned())
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();

                    Arc::new(ForkPseudoState::new(forks))
                }
                PseudoStateKind::Join => {
                    let sources = transitions_data
                        .and_then(|data| data.joins())
                        .and_then(|joins| joins.get(&id))
                        .map(|sources| {
                            sources
                                .iter()
                                .filter_map(|source| state_map.get(source).cloned())
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();

                    let targets = transitions_data
                        .map(|data| data.transitions())
                        .map(|transitions| {
                            transitions
                                .iter()
                                .filter(|transition| transition.source() == &id)
                                .filter_map(|transition| {
                                    state_map
                                        .get(transition.target())
                                        .cloned()
                                        .map(|target| JoinStateData::new(target, None))
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();

                    Arc::new(JoinPseudoState::new(vec![sources], targets))
                }
                PseudoStateKind::Entry => {
                    let target = transitions_data
                        .and_then(|data| data.entrys())
                        .and_then(|entries| entries.iter().find(|entry| entry.source() == &id))
                        .and_then(|entry| state_map.get(entry.target()).cloned());

                    let Some(target) = target else {
                        continue;
                    };
                    Arc::new(EntryPseudoState::new(target))
                }
                PseudoStateKind::Exit => {
                    let target = transitions_data
                        .and_then(|data| data.exits())
                        .and_then(|exits| exits.iter().find(|exit| exit.source() == &id))
                        .and_then(|exit| state_map.get(exit.target()).cloned());

                    let Some(target) = target else {
                        continue;
                    };
                    Arc::new(ExitPseudoState::new(StateHolder::with_state(target)))
                }
                PseudoStateKind::HistoryShallow | PseudoStateKind::HistoryDeep => {
                    let default_state = transitions_data
                        .and_then(|data| data.historys())
                        .and_then(|historys| historys.iter().find(|history| history.source() == &id))
                        .and_then(|history| state_map.get(history.target()).cloned());

                    let default_holder = StateHolder::new();
                    if let Some(default_state) = default_state {
                        default_holder.set_state(default_state);
                    }

                    Arc::new(HistoryPseudoState::new(
                        kind,
                        default_holder,
                        StateHolder::new(),
                    ))
                }
            };

            state.set_pseudo_state(Some(pseudo));
        }

        Ok(())
    }

    /// Adapts a configuration guard to the runtime guard signature.
    ///
    /// # Arguments
    /// * `guard` - the configuration guard
    fn guard_adapter(&self, guard: Arc<dyn StateMachineGuard<S, E>>) -> BoxedStateGuard<S, E>
    where
        E: Hash,
    {
        Arc::new(move |context| {
            let guard = guard.clone();
            Box::pin(async move { guard.evaluate(context).await })
        })
    }

    fn build_state_machine_internal(
        &self,
        states: Vec<Arc<dyn StateMachineState<S, E>>>,
        transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
        initial_state: Arc<dyn StateMachineState<S, E>>,
        initial_transition: Arc<dyn StateMachineTransition<S, E>>,
        extended_state: Arc<dyn ExtendedState>,
        machine_id: String,
        _uuid: Option<Uuid>,
        _state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> Result<Arc<dyn StateMachine<S, E>>, BoxError> {
        let mut machine = BaseStateMachine::new(
            states,
            transitions,
            initial_state,
            Some(initial_transition),
            None,
            extended_state,
        );
        machine.set_id(machine_id);
        machine.init()?;

        let machine = Arc::new(machine);
        let handle: Arc<dyn StateMachine<S, E>> = machine.clone();
        let weak: std::sync::Weak<dyn StateMachine<S, E>> = Arc::downgrade(&handle);
        let executor = Arc::new(
            crate::support::default_state_machine_executor::DefaultStateMachineExecutor::new(
                weak,
                None,
                machine.transitions_arc().to_vec(),
                machine.transition_conflict_policy(),
            ),
        );
        executor.set_self_arc();
        machine.attach(executor);

        // Every machine this factory builds gets the configured features, no
        // matter whether it is the top level machine, a region or a machine which
        // is used as the submachine of a state.
        self.apply_machine_features(&handle, _state_machine_model)?;

        Ok(handle)
    }

    /// Applies the startup part of the post build features to the top level
    /// machine.
    ///
    /// The listeners, interceptors and monitors are attached by
    /// [`Self::apply_machine_features`] when a machine is built.
    fn apply_post_build_features(
        &self,
        machine: Arc<dyn StateMachine<S, E>>,
        state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> Result<(), BoxError> {
        let configuration = state_machine_model.get_configuration_data();

        if (self.handle_autostartup || configuration.is_auto_start())
            && let Ok(handle) = tokio::runtime::Handle::try_current() {
                let machine = machine.clone();
                handle.spawn(async move {
                    if let Err(error) = machine.start().await {
                        tracing::warn!(%error, "auto startup of the state machine failed");
                    }
                });
            }

        Ok(())
    }

    /// Attaches the configured listeners, interceptors, monitors and the security
    /// interceptor to one machine.
    ///
    /// [`Self::build_machine`] calls it for every machine it builds, so the
    /// machines of the orthogonal regions and the machines used as submachines
    /// get them too.
    fn apply_machine_features(
        &self,
        machine: &Arc<dyn StateMachine<S, E>>,
        state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> Result<(), BoxError> {
        let configuration = state_machine_model.get_configuration_data();

        for listener in &configuration.listeners {
            machine.add_state_listener(listener.clone())?;
        }

        for interceptor in &configuration.interceptors {
            machine.add_state_machine_interceptor(interceptor.clone())?;
        }

        if let Some(monitor) = configuration.state_machine_monitor.as_ref() {
            machine.add_state_machine_monitor(monitor.clone())?;
        }

        if let Some(monitor) = self.default_state_machine_monitor.as_ref() {
            machine.add_state_machine_monitor(monitor.clone())?;
        }

        // Security rules are enforced through the interceptor chain, which the
        // machine runs around every event and transition.
        if configuration.is_security_enabled() {
            let interceptor =
                crate::security::state_machine_security_interceptor::DefaultStateMachineSecurityInterceptor::<
                    S,
                    E,
                >::new(
                    configuration.event_security_rule().cloned(),
                    configuration.transition_security_rule().cloned(),
                )
                .with_decision_managers(
                    configuration.event_security_access_decision_manager.clone(),
                    configuration.transition_security_access_decision_manager.clone(),
                );

            if interceptor.has_rules() {
                machine.add_state_machine_interceptor(Arc::new(interceptor))?;
            }
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
        let state = crate::state::object_state::ObjectState::new(id, pseudo_state)
            .with_deferred(deferred)
            .with_entry_actions(entry_actions)
            .with_exit_actions(exit_actions)
            .with_state_actions(state_actions);
        Ok(Arc::new(state))
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
        let state = crate::state::object_state::ObjectState::new(id, pseudo_state)
            .with_deferred(deferred)
            .with_entry_actions(entry_actions)
            .with_exit_actions(exit_actions)
            .with_submachine(submachine);
        Ok(Arc::new(state))
    }

    fn build_region_state_internal(
        &self,
        id: S,
        regions: Vec<Arc<dyn crate::region::Region<S, E>>>,
        deferred: Vec<E>,
        entry_actions: Vec<BoxedStateAction<S, E>>,
        exit_actions: Vec<BoxedStateAction<S, E>>,
        pseudo_state: Option<Arc<dyn PseudoState<S, E>>>,
        _state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> Result<Arc<dyn StateMachineState<S, E>>, BoxError> {
        let state = crate::state::object_state::ObjectState::new(id, pseudo_state)
            .with_deferred(deferred)
            .with_entry_actions(entry_actions)
            .with_exit_actions(exit_actions)
            .with_regions(regions);
        Ok(Arc::new(state))
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
            if let Some(parent) = self.parent_state_id(state_data)? { summary.push(format!(
                "state={:?}, parent={:?}, initial={}",
                state_data.state(),
                parent,
                state_data.is_initial()
            )) } else {
                root_states.push(format!("{:?}", state_data.state()));
                summary.push(format!(
                    "state={:?}, parent=<root>, initial={}",
                    state_data.state(),
                    state_data.is_initial()
                ));
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
        Arc::new(move |context| -> BoxFuture<'_, Result<(), BoxError>> {
            let action = action.clone();
            Box::pin(async move { action.execute(context).await })
        })
    }

    fn build_transition_trigger(
        &self,
        transition_data: &TransitionData<S, E>,
    ) -> Result<Option<Arc<dyn Trigger<S, E>>>, BoxError> {
        if let Some(event) = transition_data.event().cloned() {
            return Ok(Some(event_trigger::<S, E>(event)));
        }

        if transition_data.period().is_some() || transition_data.count().is_some() {
            let period = Duration::from_millis(transition_data.period().unwrap_or(0));
            let count = transition_data.count().unwrap_or(0) as usize;

            return Ok(Some(Arc::new(
                crate::trigger::timer_trigger::TimerTrigger::<S, E>::with_count(period, count),
            )));
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
                match stack.pop() {
                    Some(state) => grouped.push(state),
                    None => break,
                }
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
    E: Clone + Eq + Hash + Send + Sync + 'static,
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
// The tests build a configuration field by field, which reads clearer than one
// long struct literal.
#[allow(clippy::field_reassign_with_default)]
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
        let child = StateData::with_hierarchy(Some(Arc::new(1) as Arc<dyn Any + Send + Sync>), None, 2, false);
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

        let child_a = StateData::with_hierarchy(Some(Arc::new(1) as Arc<dyn Any + Send + Sync>), None, 10, true);
        let child_b = StateData::with_hierarchy(Some(Arc::new(1) as Arc<dyn Any + Send + Sync>), None, 11, false);
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
                    Some(Arc::new(1) as Arc<dyn Any + Send + Sync>),
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
                vec![Arc::new(|_context| {
                    Box::pin(async move { Ok(()) })
                })],
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
        block_on(machine.send_event(Arc::new(GenericMessage::with_payload(100)))).unwrap();

        assert_eq!(
            machine.state().map(|state| *state.id()),
            Some(2)
        );
    }

    /// A guard which always allows the branch it selects.
    struct AlwaysAllow;

    #[next_web_core::async_trait]
    impl crate::config::guard::StateMachineGuard<i32, i32> for AlwaysAllow {
        async fn evaluate(
            &self,
            _ctx: &dyn crate::state_context::StateContext<i32, i32>,
        ) -> bool {
            true
        }
    }

    #[test]
    fn choice_pseudo_state_is_wired_from_the_model() {
        use crate::config::model::choice_data::ChoiceData;
        use crate::state::pseudo_state_kind::PseudoStateKind;
        use std::collections::HashMap;

        let mut configuration = ConfigurationData::default();
        configuration.verifier_enabled = false;

        let mut choice_state = StateData::<i32, i32>::new(2);
        choice_state.set_pseudo_state_kind(PseudoStateKind::Choice);

        let mut transitions_data =
            TransitionsData::new(vec![TransitionData::new(1, 2, 100)]);
        let mut choices: HashMap<i32, Vec<ChoiceData<i32, i32>>> = HashMap::new();
        choices.insert(
            2,
            vec![ChoiceData::new(2, 4, Arc::new(AlwaysAllow))],
        );
        transitions_data.set_choices(choices);

        let model = DefaultStateMachineModel::new(
            configuration,
            Some(
                vec![
                    StateData::<i32, i32>::with_initial(1, true),
                    choice_state,
                    StateData::<i32, i32>::new(3),
                    StateData::<i32, i32>::new(4),
                ]
                .into(),
            ),
            Some(transitions_data),
        );

        let factory = BaseStateMachineFactory::new(Arc::new(model), None);
        let machine = factory.get_state_machine().expect("machine builds");

        block_on(machine.start()).unwrap();
        block_on(machine.send_event(Arc::new(GenericMessage::with_payload(100)))).unwrap();

        assert_eq!(
            machine.state().map(|state| *state.id()),
            Some(4),
            "the choice pseudo state continues with its selected target"
        );
    }

    #[test]
    fn orthogonal_regions_are_assembled_from_region_keys() {
        use std::any::Any;

        let mut configuration = ConfigurationData::default();
        configuration.verifier_enabled = false;

        let parent = Arc::new(2) as Arc<dyn Any + Send + Sync>;
        let states = vec![
            StateData::<i32, i32>::with_initial(1, true),
            StateData::<i32, i32>::new(2),
            StateData::<i32, i32>::with_hierarchy(
                Some(parent.clone()),
                Some("r1".to_owned()),
                11,
                true,
            ),
            StateData::<i32, i32>::with_hierarchy(
                Some(parent.clone()),
                Some("r1".to_owned()),
                12,
                false,
            ),
            StateData::<i32, i32>::with_hierarchy(
                Some(parent.clone()),
                Some("r2".to_owned()),
                21,
                true,
            ),
            StateData::<i32, i32>::with_hierarchy(
                Some(parent),
                Some("r2".to_owned()),
                22,
                false,
            ),
        ];

        let to_parent = TransitionData::new(1, 2, 100);
        let mut region1 = TransitionData::new(11, 12, 200);
        region1.set_state(2);
        let mut region2 = TransitionData::new(21, 22, 201);
        region2.set_state(2);

        let model = DefaultStateMachineModel::new(
            configuration,
            Some(states.into()),
            Some(TransitionsData::new(vec![to_parent, region1, region2])),
        );

        let factory = BaseStateMachineFactory::new(Arc::new(model), None);
        let machine = factory
            .get_state_machine()
            .expect("the orthogonal machine builds");

        block_on(machine.start()).unwrap();
        assert_eq!(machine.state().map(|state| *state.id()), Some(1));

        block_on(machine.send_event(Arc::new(GenericMessage::with_payload(100)))).unwrap();
        let current = machine.state().expect("the machine has a current state");
        assert_eq!(current.id(), &2);
        assert!(
            current.contains_id(&11),
            "the first region starts at its initial state"
        );
        assert!(
            current.contains_id(&21),
            "the second region starts at its initial state"
        );

        block_on(machine.send_event(Arc::new(GenericMessage::with_payload(200)))).unwrap();
        let current = machine.state().expect("the machine has a current state");
        assert!(current.contains_id(&12));
        assert!(!current.contains_id(&11));

        block_on(machine.send_event(Arc::new(GenericMessage::with_payload(201)))).unwrap();
        let current = machine.state().expect("the machine has a current state");
        assert!(current.contains_id(&22));
        assert!(!current.contains_id(&21));
    }

    #[test]
    fn configured_monitor_also_observes_the_region_machines() {
        use crate::monitor::state_machine_monitor::StateMachineMonitor;
        use crate::transition::StateMachineTransition;
        use std::any::Any;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;

        #[derive(Default)]
        struct RegionMonitor {
            transitions: AtomicUsize,
        }

        impl StateMachineMonitor<i32, i32> for RegionMonitor {
            fn transition(
                &self,
                _state_machine: &dyn crate::StateMachine<i32, i32>,
                _transition: &dyn StateMachineTransition<i32, i32>,
                _duration: Duration,
            ) {
                self.transitions.fetch_add(1, Ordering::SeqCst);
            }

            fn action(
                &self,
                _state_machine: &dyn crate::StateMachine<i32, i32>,
                _action: &crate::BoxedStateAction<i32, i32>,
                _duration: Duration,
            ) {
            }
        }

        let monitor = Arc::new(RegionMonitor::default());

        let mut configuration = ConfigurationData::default();
        configuration.verifier_enabled = false;
        configuration.state_machine_monitor = Some(monitor.clone());

        let parent = Arc::new(2) as Arc<dyn Any + Send + Sync>;
        let states = vec![
            StateData::<i32, i32>::with_initial(1, true),
            StateData::<i32, i32>::new(2),
            StateData::<i32, i32>::with_hierarchy(
                Some(parent.clone()),
                Some("r1".to_owned()),
                11,
                true,
            ),
            StateData::<i32, i32>::with_hierarchy(
                Some(parent.clone()),
                Some("r1".to_owned()),
                12,
                false,
            ),
            StateData::<i32, i32>::with_hierarchy(
                Some(parent.clone()),
                Some("r2".to_owned()),
                21,
                true,
            ),
            StateData::<i32, i32>::with_hierarchy(
                Some(parent),
                Some("r2".to_owned()),
                22,
                false,
            ),
        ];

        let to_parent = TransitionData::new(1, 2, 100);
        let mut region1 = TransitionData::new(11, 12, 200);
        region1.set_state(2);
        let mut region2 = TransitionData::new(21, 22, 201);
        region2.set_state(2);

        let model = DefaultStateMachineModel::new(
            configuration,
            Some(states.into()),
            Some(TransitionsData::new(vec![to_parent, region1, region2])),
        );

        let factory = BaseStateMachineFactory::new(Arc::new(model), None);
        let machine = factory
            .get_state_machine()
            .expect("the orthogonal machine builds");

        block_on(machine.start()).unwrap();
        block_on(machine.send_event(Arc::new(GenericMessage::with_payload(100)))).unwrap();
        assert_eq!(
            monitor.transitions.load(Ordering::SeqCst),
            1,
            "only the top level transition is reported so far"
        );

        // The region transitions are run by the region machines, which only
        // report them when the configured monitor is attached to them as well.
        block_on(machine.send_event(Arc::new(GenericMessage::with_payload(200)))).unwrap();
        assert_eq!(
            monitor.transitions.load(Ordering::SeqCst),
            2,
            "the first region transition must reach the monitor"
        );

        block_on(machine.send_event(Arc::new(GenericMessage::with_payload(201)))).unwrap();
        assert_eq!(
            monitor.transitions.load(Ordering::SeqCst),
            3,
            "the second region transition must reach the monitor"
        );
    }

    #[test]
    fn configured_interceptor_also_observes_the_submachine() {
        use crate::state::StateMachineState;
        use crate::state_context::StateContext;
        use crate::support::state_machine_interceptor::StateMachineInterceptor;
        use crate::transition::StateMachineTransition;
        use next_web_core::error::BoxError;
        use std::any::Any;
        use std::sync::atomic::{AtomicUsize, Ordering};

        #[derive(Default)]
        struct SubmachineInterceptor {
            events: AtomicUsize,
        }

        impl StateMachineInterceptor<i32, i32> for SubmachineInterceptor {
            fn pre_event(
                &self,
                message: Arc<dyn next_web_core::messaging::Message<i32>>,
                _state_machine: &dyn crate::StateMachine<i32, i32>,
            ) -> Result<Arc<dyn next_web_core::messaging::Message<i32>>, BoxError> {
                self.events.fetch_add(1, Ordering::SeqCst);

                Ok(message)
            }

            fn pre_state_change(
                &self,
                _state: &dyn StateMachineState<i32, i32>,
                _message: Option<&dyn next_web_core::messaging::Message<i32>>,
                _transition: Option<&dyn StateMachineTransition<i32, i32>>,
                _state_machine: &dyn crate::StateMachine<i32, i32>,
                _root_state_machine: &dyn crate::StateMachine<i32, i32>,
            ) -> Result<(), BoxError> {
                Ok(())
            }

            fn post_state_change(
                &self,
                _state: &dyn StateMachineState<i32, i32>,
                _message: Option<&dyn next_web_core::messaging::Message<i32>>,
                _transition: Option<&dyn StateMachineTransition<i32, i32>>,
                _state_machine: &dyn crate::StateMachine<i32, i32>,
                _root_state_machine: &dyn crate::StateMachine<i32, i32>,
            ) {
            }

            fn pre_transition(
                &self,
                _state_context: &dyn StateContext<i32, i32>,
            ) -> Result<(), BoxError> {
                Ok(())
            }

            fn post_transition(
                &self,
                _state_context: &dyn StateContext<i32, i32>,
            ) -> Result<(), BoxError> {
                Ok(())
            }

            fn state_machine_error(
                &self,
                _state_machine: &dyn crate::StateMachine<i32, i32>,
                error: BoxError,
            ) -> Option<BoxError> {
                Some(error)
            }
        }

        let interceptor = Arc::new(SubmachineInterceptor::default());

        let mut configuration = ConfigurationData::default();
        configuration.verifier_enabled = false;
        configuration.interceptors = vec![interceptor.clone()];

        let parent = Arc::new(2) as Arc<dyn Any + Send + Sync>;
        let states = vec![
            StateData::<i32, i32>::with_initial(1, true),
            StateData::<i32, i32>::new(2),
            StateData::<i32, i32>::with_hierarchy(Some(parent.clone()), None, 21, true),
            StateData::<i32, i32>::with_hierarchy(Some(parent), None, 22, false),
        ];

        let model = DefaultStateMachineModel::new(
            configuration,
            Some(states.into()),
            Some(TransitionsData::new(vec![
                TransitionData::new(1, 2, 100),
                TransitionData::new(21, 22, 200),
            ])),
        );

        let factory = BaseStateMachineFactory::new(Arc::new(model), None);
        let machine = factory.get_state_machine().expect("the machine builds");

        block_on(machine.start()).unwrap();
        block_on(machine.send_event(Arc::new(GenericMessage::with_payload(100)))).unwrap();
        assert_eq!(
            interceptor.events.load(Ordering::SeqCst),
            1,
            "the top level machine runs the interceptor"
        );

        let current_state = machine.state().expect("the machine has a current state");
        assert!(
            current_state.contains_id(&21),
            "entering state 2 starts the submachine on its initial state"
        );

        // The event is sent to the state which forwards it to its machine.
        block_on(current_state.send_event(Arc::new(GenericMessage::with_payload(200))));

        assert_eq!(
            interceptor.events.load(Ordering::SeqCst),
            2,
            "the submachine runs the configured interceptor too"
        );
    }

    #[test]
    fn the_machine_offers_an_event_to_its_submachine() {
        use crate::monitor::state_machine_monitor::StateMachineMonitor;
        use crate::transition::StateMachineTransition;
        use std::any::Any;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;

        #[derive(Default)]
        struct NestedMonitor {
            transitions: AtomicUsize,
        }

        impl StateMachineMonitor<i32, i32> for NestedMonitor {
            fn transition(
                &self,
                _state_machine: &dyn crate::StateMachine<i32, i32>,
                _transition: &dyn StateMachineTransition<i32, i32>,
                _duration: Duration,
            ) {
                self.transitions.fetch_add(1, Ordering::SeqCst);
            }

            fn action(
                &self,
                _state_machine: &dyn crate::StateMachine<i32, i32>,
                _action: &crate::BoxedStateAction<i32, i32>,
                _duration: Duration,
            ) {
            }
        }

        let monitor = Arc::new(NestedMonitor::default());

        let mut configuration = ConfigurationData::default();
        configuration.verifier_enabled = false;
        configuration.state_machine_monitor = Some(monitor.clone());

        let parent = Arc::new(2) as Arc<dyn Any + Send + Sync>;
        let states = vec![
            StateData::<i32, i32>::with_initial(1, true),
            StateData::<i32, i32>::new(2),
            StateData::<i32, i32>::with_hierarchy(Some(parent.clone()), None, 21, true),
            StateData::<i32, i32>::with_hierarchy(Some(parent), None, 22, false),
        ];

        let model = DefaultStateMachineModel::new(
            configuration,
            Some(states.into()),
            Some(TransitionsData::new(vec![
                TransitionData::new(1, 2, 100),
                // The transition of the nested machine is declared without a
                // scope, so it belongs to the group which owns both endpoints.
                TransitionData::new(21, 22, 200),
            ])),
        );

        let factory = BaseStateMachineFactory::new(Arc::new(model), None);
        let machine = factory.get_state_machine().expect("the machine builds");

        block_on(machine.start()).unwrap();
        block_on(machine.send_event(Arc::new(GenericMessage::with_payload(100)))).unwrap();
        assert_eq!(monitor.transitions.load(Ordering::SeqCst), 1);

        // The top level machine has no transition for 200, so it offers the
        // event to the machine behind state 2.
        let result =
            block_on(machine.send_event(Arc::new(GenericMessage::with_payload(200)))).unwrap();

        assert_eq!(
            result.result_type(),
            crate::state_machine_event_result::ResultType::Accepted,
            "the nested machine accepted the event"
        );
        assert!(
            machine
                .state()
                .expect("the machine has a current state")
                .contains_id(&22),
            "the nested machine ran its transition"
        );
        assert_eq!(
            monitor.transitions.load(Ordering::SeqCst),
            2,
            "the transition of the nested machine reached the monitor"
        );
    }

    #[test]
    fn fork_pseudo_state_is_wired_from_the_model() {
        use crate::state::pseudo_state_kind::PseudoStateKind;
        use std::collections::HashMap;

        let mut configuration = ConfigurationData::default();
        configuration.verifier_enabled = false;

        let mut fork_state = StateData::<i32, i32>::new(2);
        fork_state.set_pseudo_state_kind(PseudoStateKind::Fork);

        let mut transitions_data =
            TransitionsData::new(vec![TransitionData::new(1, 2, 100)]);
        let mut forks: HashMap<i32, Vec<i32>> = HashMap::new();
        forks.insert(2, vec![3, 4]);
        transitions_data.set_forks(forks);

        let model = DefaultStateMachineModel::new(
            configuration,
            Some(
                vec![
                    StateData::<i32, i32>::with_initial(1, true),
                    fork_state,
                    StateData::<i32, i32>::new(3),
                    StateData::<i32, i32>::new(4),
                ]
                .into(),
            ),
            Some(transitions_data),
        );

        let factory = BaseStateMachineFactory::new(Arc::new(model), None);
        let machine = factory.get_state_machine().expect("machine builds");

        block_on(machine.start()).unwrap();
        block_on(machine.send_event(Arc::new(GenericMessage::with_payload(100)))).unwrap();

        assert_eq!(
            machine.state().map(|state| *state.id()),
            Some(4),
            "the last fork target stays current"
        );
    }

    #[test]
    fn event_security_rule_vetoes_events_without_authority() {
        use crate::security::security_rule::SecurityRule;
        use crate::security::state_machine_security_interceptor::HEADER_AUTHENTICATION;
        use crate::state_machine_event_result::ResultType;
        use next_web_core::any::AnyValue;
        use next_web_core::messaging::message_headers::MessageHeaders;
        use std::collections::{HashMap, HashSet};

        let mut configuration = ConfigurationData::default();
        configuration.verifier_enabled = false;
        configuration.security_enabled = true;
        configuration.event_security_rule =
            Some(SecurityRule::new().with_attributes(HashSet::from(["admin".to_owned()])));

        let model = DefaultStateMachineModel::new(
            configuration,
            Some(
                vec![
                    StateData::<i32, i32>::with_initial(1, true),
                    StateData::<i32, i32>::new(2),
                ]
                .into(),
            ),
            Some(TransitionsData::new(vec![TransitionData::new(1, 2, 100)])),
        );

        let factory = BaseStateMachineFactory::new(Arc::new(model), None);
        let machine = factory.get_state_machine().expect("machine builds");
        block_on(machine.start()).unwrap();

        let denied =
            block_on(machine.send_event(Arc::new(GenericMessage::with_payload(100)))).unwrap();
        assert_eq!(denied.result_type(), ResultType::Denied);
        assert_eq!(machine.state().map(|state| *state.id()), Some(1));

        let mut headers: HashMap<String, AnyValue> = HashMap::new();
        headers.insert(HEADER_AUTHENTICATION.to_owned(), AnyValue::from("admin"));

        let accepted = block_on(machine.send_event(Arc::new(GenericMessage::new(
            Some(100),
            MessageHeaders::from(headers),
        ))))
        .unwrap();
        assert_eq!(accepted.result_type(), ResultType::Accepted);
        assert_eq!(machine.state().map(|state| *state.id()), Some(2));
    }

    #[tokio::test]
    async fn timer_transition_fires_from_the_model() {
        use crate::transition::transition_kind::TransitionKind;
        use std::time::Duration;

        let mut configuration = ConfigurationData::default();
        configuration.verifier_enabled = false;

        let states = vec![
            StateData::<i32, i32>::with_initial(1, true),
            StateData::<i32, i32>::new(2),
        ];

        // A transition without an event fires from a timer: 10ms period, one
        // firing.
        let timer = TransitionData::with_all(
            1,
            2,
            None,
            None,
            Some(10),
            Some(1),
            Vec::new(),
            None,
            TransitionKind::External,
            None,
            "",
        );

        let model = DefaultStateMachineModel::new(
            configuration,
            Some(states.into()),
            Some(TransitionsData::new(vec![timer])),
        );

        let factory = BaseStateMachineFactory::new(Arc::new(model), None);
        let machine = factory.get_state_machine().expect("machine builds");

        machine.start().await.expect("start succeeds");
        assert_eq!(machine.state().map(|state| *state.id()), Some(1));

        tokio::time::sleep(Duration::from_millis(150)).await;

        assert_eq!(
            machine.state().map(|state| *state.id()),
            Some(2),
            "the timer moved the machine"
        );
    }

    #[tokio::test]
    async fn configured_monitor_and_interceptor_are_attached() {
        use crate::monitor::state_machine_monitor::StateMachineMonitor;
        use crate::state::StateMachineState;
        use crate::state_context::StateContext;
        use crate::support::state_machine_interceptor::StateMachineInterceptor;
        use crate::transition::StateMachineTransition;
        use next_web_core::error::BoxError;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;

        #[derive(Default)]
        struct CountingMonitor {
            transitions: AtomicUsize,
            actions: AtomicUsize,
        }

        impl StateMachineMonitor<i32, i32> for CountingMonitor {
            fn transition(
                &self,
                _state_machine: &dyn crate::StateMachine<i32, i32>,
                _transition: &dyn StateMachineTransition<i32, i32>,
                _duration: Duration,
            ) {
                self.transitions.fetch_add(1, Ordering::SeqCst);
            }

            fn action(
                &self,
                _state_machine: &dyn crate::StateMachine<i32, i32>,
                _action: &crate::BoxedStateAction<i32, i32>,
                _duration: Duration,
            ) {
                self.actions.fetch_add(1, Ordering::SeqCst);
            }
        }

        #[derive(Default)]
        struct CountingInterceptor {
            events: AtomicUsize,
            transitions: AtomicUsize,
        }

        impl StateMachineInterceptor<i32, i32> for CountingInterceptor {
            fn pre_event(
                &self,
                message: Arc<dyn next_web_core::messaging::Message<i32>>,
                _state_machine: &dyn crate::StateMachine<i32, i32>,
            ) -> Result<Arc<dyn next_web_core::messaging::Message<i32>>, BoxError> {
                self.events.fetch_add(1, Ordering::SeqCst);
                Ok(message)
            }

            fn pre_state_change(
                &self,
                _state: &dyn StateMachineState<i32, i32>,
                _message: Option<&dyn next_web_core::messaging::Message<i32>>,
                _transition: Option<&dyn StateMachineTransition<i32, i32>>,
                _state_machine: &dyn crate::StateMachine<i32, i32>,
                _root_state_machine: &dyn crate::StateMachine<i32, i32>,
            ) -> Result<(), BoxError> {
                Ok(())
            }

            fn post_state_change(
                &self,
                _state: &dyn StateMachineState<i32, i32>,
                _message: Option<&dyn next_web_core::messaging::Message<i32>>,
                _transition: Option<&dyn StateMachineTransition<i32, i32>>,
                _state_machine: &dyn crate::StateMachine<i32, i32>,
                _root_state_machine: &dyn crate::StateMachine<i32, i32>,
            ) {
            }

            fn pre_transition(
                &self,
                _state_context: &dyn StateContext<i32, i32>,
            ) -> Result<(), BoxError> {
                self.transitions.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }

            fn post_transition(
                &self,
                _state_context: &dyn StateContext<i32, i32>,
            ) -> Result<(), BoxError> {
                Ok(())
            }

            fn state_machine_error(
                &self,
                _state_machine: &dyn crate::StateMachine<i32, i32>,
                error: BoxError,
            ) -> Option<BoxError> {
                Some(error)
            }
        }

        let monitor = Arc::new(CountingMonitor::default());
        let interceptor = Arc::new(CountingInterceptor::default());

        let mut configuration = ConfigurationData::default();
        configuration.verifier_enabled = false;
        configuration.state_machine_monitor = Some(monitor.clone());
        configuration.interceptors = vec![interceptor.clone()];

        let states = vec![
            StateData::<i32, i32>::with_initial(1, true),
            StateData::<i32, i32>::new(2),
        ];

        let action: crate::BoxedStateAction<i32, i32> =
            Arc::new(|_context| Box::pin(async move { Ok(()) }));
        let transition = TransitionData::with_all(
            1,
            2,
            None,
            Some(100),
            None,
            None,
            vec![action],
            None,
            crate::transition::transition_kind::TransitionKind::External,
            None,
            "",
        );

        let model = DefaultStateMachineModel::new(
            configuration,
            Some(states.into()),
            Some(TransitionsData::new(vec![transition])),
        );

        let factory = BaseStateMachineFactory::new(Arc::new(model), None);
        let machine = factory.get_state_machine().expect("machine builds");

        machine.start().await.expect("start succeeds");
        machine
            .send_event(Arc::new(GenericMessage::with_payload(100)))
            .await
            .expect("event is sent");

        assert_eq!(interceptor.events.load(Ordering::SeqCst), 1);
        assert_eq!(interceptor.transitions.load(Ordering::SeqCst), 1);
        assert_eq!(monitor.transitions.load(Ordering::SeqCst), 1);
        assert_eq!(monitor.actions.load(Ordering::SeqCst), 1);
    }

    /// A decision manager which denies everything.
    struct DenyAll;

    impl crate::security::access_decision_manager::AccessDecisionManager for DenyAll {
        fn decide(
            &self,
            _authentication: &next_web_core::any::AnyValue,
            _value: &next_web_core::any::AnyValue,
            _config_attributes: Vec<
                Box<dyn crate::security::access_decision_manager::ConfigAttribute>,
            >,
        ) -> Result<(), next_web_core::error::BoxError> {
            Err(Box::new(std::io::Error::other("denied by policy")))
        }

        fn supports(
            &self,
            _attribute: &dyn crate::security::access_decision_manager::ConfigAttribute,
        ) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn custom_decision_manager_vetoes_the_event() {
        use crate::security::security_rule::SecurityRule;
        use crate::security::state_machine_security_interceptor::HEADER_AUTHENTICATION;
        use crate::state_machine_event_result::ResultType;
        use next_web_core::any::AnyValue;
        use next_web_core::messaging::message_headers::MessageHeaders;
        use std::collections::{HashMap, HashSet};

        let mut configuration = ConfigurationData::default();
        configuration.verifier_enabled = false;
        configuration.security_enabled = true;
        configuration.event_security_rule =
            Some(SecurityRule::new().with_attributes(HashSet::from(["admin".to_owned()])));
        configuration.event_security_access_decision_manager = Some(Arc::new(DenyAll));

        let model = DefaultStateMachineModel::new(
            configuration,
            Some(
                vec![
                    StateData::<i32, i32>::with_initial(1, true),
                    StateData::<i32, i32>::new(2),
                ]
                .into(),
            ),
            Some(TransitionsData::new(vec![TransitionData::new(1, 2, 100)])),
        );

        let factory = BaseStateMachineFactory::new(Arc::new(model), None);
        let machine = factory.get_state_machine().expect("machine builds");
        machine.start().await.expect("start succeeds");

        // The authority is present, but the configured manager is the one which
        // decides, and it denies.
        let mut headers: HashMap<String, AnyValue> = HashMap::new();
        headers.insert(HEADER_AUTHENTICATION.to_owned(), AnyValue::from("admin"));

        let denied = machine
            .send_event(Arc::new(GenericMessage::new(
                Some(100),
                MessageHeaders::from(headers),
            )))
            .await
            .expect("event is sent");

        assert_eq!(denied.result_type(), ResultType::Denied);
        assert_eq!(machine.state().map(|state| *state.id()), Some(1));
    }

    #[tokio::test]
    async fn expression_rule_denies_without_an_evaluator() {
        use crate::security::security_rule::SecurityRule;
        use crate::security::state_machine_security_interceptor::HEADER_AUTHENTICATION;
        use crate::state_machine_event_result::ResultType;
        use next_web_core::any::AnyValue;
        use next_web_core::messaging::message_headers::MessageHeaders;
        use std::collections::HashMap;

        let mut configuration = ConfigurationData::default();
        configuration.verifier_enabled = false;
        configuration.security_enabled = true;
        configuration.event_security_rule =
            Some(SecurityRule::new().with_expression("hasRole('admin')"));

        let model = DefaultStateMachineModel::new(
            configuration,
            Some(
                vec![
                    StateData::<i32, i32>::with_initial(1, true),
                    StateData::<i32, i32>::new(2),
                ]
                .into(),
            ),
            Some(TransitionsData::new(vec![TransitionData::new(1, 2, 100)])),
        );

        let factory = BaseStateMachineFactory::new(Arc::new(model), None);
        let machine = factory.get_state_machine().expect("machine builds");
        machine.start().await.expect("start succeeds");

        let mut headers: HashMap<String, AnyValue> = HashMap::new();
        headers.insert(HEADER_AUTHENTICATION.to_owned(), AnyValue::from("admin"));

        let denied = machine
            .send_event(Arc::new(GenericMessage::new(
                Some(100),
                MessageHeaders::from(headers),
            )))
            .await
            .expect("event is sent");

        assert_eq!(
            denied.result_type(),
            ResultType::Denied,
            "an expression rule fails closed without an evaluator"
        );
    }
}
