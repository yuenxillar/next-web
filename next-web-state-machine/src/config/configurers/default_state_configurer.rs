use std::{any::Any, cell::RefCell, collections::HashMap, hash::Hash, rc::Rc, sync::Arc};

use next_web_core::error::BoxError;
use uuid::Uuid;

use crate::{
    config::{
        action::{actions::Actions, StateMachineAction},
        builders::{
            state_machine_state_builder::StateMachineStateBuilder,
            state_machine_state_configurer::StateMachineStateConfigurer,
        },
        common::{
            builder::Builder,
            configurer::Configurer,
            configurer_adapter::{ConfigurerAdapter, ConfigurerAdapterExt},
        },
        configurer_builder::ConfigurerBuilder,
        configurers::state_configurer::{History, StateConfigurer},
        model::{state_data::StateData, states_data::StatesData},
        state_machine_factory::StateMachineFactory,
    },
    state::pseudo_state_kind::PseudoStateKind,
    BoxedStateAction, StateMachine,
};

/// Default implementation of StateConfigurer
pub struct DefaultStateConfigurer<S, E>
where
    S: Eq + Hash,
    S: Clone + 'static,
    E: Clone + 'static,
{
    shared: Rc<RefCell<DefaultStateConfigurerData<S, E>>>,

    pub(crate) base: ConfigurerAdapter<
        StatesData<S, E>,
        Box<dyn StateMachineStateConfigurer<S, E>>,
        StateMachineStateBuilder<S, E>,
    >,
}

struct DefaultStateConfigurerData<S, E>
where
    S: Eq + Hash,
    S: Clone + 'static,
    E: Clone + 'static,
{
    parent: Option<S>,
    region: String,
    incomplete: HashMap<S, StateData<S, E>>,
    initial_state: Option<S>,
    initial_action: Option<Arc<dyn StateMachineAction<S, E>>>,
    ends: Vec<S>,
    history: Option<S>,
    history_type: Option<History>,
    choices: Vec<S>,
    junctions: Vec<S>,
    forks: Vec<S>,
    joins: Vec<S>,
    exits: Vec<S>,
    entrys: Vec<S>,
    submachines: HashMap<S, Arc<dyn StateMachine<S, E>>>,
    submachine_factories: HashMap<S, Arc<dyn StateMachineFactory<S, E>>>,
}

impl<S, E> DefaultStateConfigurer<S, E>
where
    S: Eq + Hash,
    S: Clone + 'static,
    E: Clone + 'static,
{
}

impl<S, E> ConfigurerBuilder<Box<dyn StateMachineStateConfigurer<S, E>>>
    for DefaultStateConfigurer<S, E>
where
    S: Eq + Hash,
    S: Send + Sync,
    E: Send + Sync,
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn and(&mut self) -> Box<dyn StateMachineStateConfigurer<S, E>> {
        todo!()
    }
}

impl<S, E> Builder<StatesData<S, E>> for DefaultStateConfigurer<S, E>
where
    S: Eq + Hash,
    S: Send + Sync,
    E: Send + Sync,
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn build(&mut self) -> Result<StatesData<S, E>, BoxError> {
        todo!()
    }
}

impl<S, E> DefaultStateConfigurer<S, E>
where
    S: Eq + Hash + Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn apply_to_builder(&self, builder: &StateMachineStateBuilder<S, E>) {
        let data = self.shared.borrow();
        let mut state_datas = Vec::new();

        for mut state_data in data.incomplete.values().cloned().collect::<Vec<_>>() {
            if state_data.parent().is_none() {
                if let Some(parent) = data.parent.as_ref() {
                    state_data.set_parent(Arc::new(parent.clone()));
                }
            }

            if let Some(initial_state) = &data.initial_state {
                if state_data.state() == initial_state {
                    state_data.set_initial(true);

                    data.initial_action.as_ref().map(|action| {
                        state_data.set_initial_action(action.clone());
                    });
                }
            }

            if data.ends.contains(&state_data.state) {
                state_data.set_end(true);
            }

            if data.choices.contains(&state_data.state) {
                state_data.set_pseudo_state_kind(PseudoStateKind::Choice);
            } else if data.junctions.contains(&state_data.state) {
                state_data.set_pseudo_state_kind(PseudoStateKind::Junction);
            } else if data.forks.contains(&state_data.state) {
                state_data.set_pseudo_state_kind(PseudoStateKind::Fork);
            } else if data.joins.contains(&state_data.state) {
                state_data.set_pseudo_state_kind(PseudoStateKind::Join);
            } else if data.entrys.contains(&state_data.state) {
                state_data.set_pseudo_state_kind(PseudoStateKind::Entry);
            } else if data.exits.contains(&state_data.state) {
                state_data.set_pseudo_state_kind(PseudoStateKind::Exit);
            }

            if let Some(history) = &data.history {
                if state_data.state() == history {
                    if let Some(history_type) = data.history_type.as_ref() {
                        state_data.set_pseudo_state_kind(match history_type {
                            History::Shallow => PseudoStateKind::HistoryShallow,
                            History::Deep => PseudoStateKind::HistoryDeep,
                        });
                    }
                }
            }

            if let Some(submachine) = data.submachines.get(state_data.state()) {
                state_data.set_submachine(submachine.clone());
            }

            if let Some(factory) = data.submachine_factories.get(state_data.state()) {
                state_data.set_submachine_factory(factory.clone());
            }

            state_datas.push(state_data);
        }

        builder.add_state_data(state_datas);
    }
}

impl<S, E> ConfigurerAdapterExt<StatesData<S, E>, StateMachineStateBuilder<S, E>>
    for DefaultStateConfigurer<S, E>
where
    S: Eq + Hash,
    S: Clone,
    S: Send + Sync,
    E: Send + Sync,
    E: Clone,
    S: 'static,
    E: 'static,
{
    fn configure(&mut self, builder: &mut StateMachineStateBuilder<S, E>) -> Result<(), BoxError> {
        self.apply_to_builder(builder);
        Ok(())
    }
}

impl<S, E> Configurer<StatesData<S, E>, StateMachineStateBuilder<S, E>>
    for DefaultStateConfigurer<S, E>
where
    S: Eq + Hash + Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn init(&mut self, _builder: &StateMachineStateBuilder<S, E>) -> Result<(), BoxError> {
        Ok(())
    }

    fn configure(&mut self, builder: &StateMachineStateBuilder<S, E>) -> Result<(), BoxError> {
        self.apply_to_builder(builder);
        Ok(())
    }

    fn is_assignable(&self, _builder: &StateMachineStateBuilder<S, E>) -> bool {
        true
    }
}

impl<S, E> StateConfigurer<S, E> for DefaultStateConfigurer<S, E>
where
    S: Eq + Hash,
    S: Clone,
    S: Send + Sync + 'static,
    E: Clone,
    E: Send + Sync + 'static,
{
    fn initial(&mut self, initial: S) -> &mut dyn StateConfigurer<S, E> {
        self.shared.borrow_mut().initial_state = Some(initial.clone());
        self.state(initial);
        self
    }

    fn initial_with_action(
        &mut self,
        initial: S,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn StateConfigurer<S, E> {
        self.shared.borrow_mut().initial_action = Some(action);
        self.initial(initial)
    }

    fn parent(&mut self, state: S) -> &mut dyn StateConfigurer<S, E> {
        self.shared.borrow_mut().parent = Some(state);
        self
    }

    fn region(&mut self, id: String) -> &mut dyn StateConfigurer<S, E> {
        self.shared.borrow_mut().region = id;
        self
    }

    fn end(&mut self, end: S) -> &mut dyn StateConfigurer<S, E> {
        self.shared.borrow_mut().ends.push(end.clone());
        self.state(end);
        self
    }

    fn state(&mut self, state: S) -> &mut dyn StateConfigurer<S, E> {
        self.state_deferred(state, Vec::new())
    }

    fn state_with_machine(
        &mut self,
        state: S,
        state_machine: Arc<dyn StateMachine<S, E>>,
    ) -> &mut dyn StateConfigurer<S, E> {
        self.state(state.clone());
        self.shared
            .borrow_mut()
            .submachines
            .insert(state, state_machine);
        self
    }

    fn state_with_factory(
        &mut self,
        state: S,
        state_machine_factory: Arc<dyn StateMachineFactory<S, E>>,
    ) -> &mut dyn StateConfigurer<S, E> {
        self.state(state.clone());
        self.shared
            .borrow_mut()
            .submachine_factories
            .insert(state, state_machine_factory);
        self
    }

    fn state_with_actions(
        &mut self,
        state: S,
        state_actions: Vec<Arc<dyn StateMachineAction<S, E>>>,
    ) -> &mut dyn StateConfigurer<S, E> {
        let r_state_actions = if !state_actions.is_empty() {
            state_actions
                .into_iter()
                .map(|action| Actions::from(action))
                .collect()
        } else {
            Vec::new()
        };

        self.add_incomplete(
            self.current_parent(),
            state,
            None,
            None,
            None,
            Some(r_state_actions),
        );
        self
    }

    fn state_with_action(
        &mut self,
        state: S,
        state_action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn StateConfigurer<S, E> {
        self.state_with_actions(state, vec![state_action])
    }

    fn state_do(
        &mut self,
        state: S,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn StateConfigurer<S, E> {
        self.state_do_with_error(state, action, None)
    }

    fn state_do_function(
        &mut self,
        state: S,
        state_action: BoxedStateAction<S, E>,
    ) -> &mut dyn StateConfigurer<S, E> {
        self.add_incomplete(
            self.current_parent(),
            state,
            None,
            None,
            None,
            Some(vec![state_action]),
        );
        self
    }

    fn state_entry_function(
        &mut self,
        state: S,
        action: BoxedStateAction<S, E>,
    ) -> &mut dyn StateConfigurer<S, E> {
        self.add_incomplete(
            self.current_parent(),
            state,
            None,
            Some(vec![action]),
            None,
            None,
        );
        self
    }

    fn state_exit_function(
        &mut self,
        state: S,
        action: BoxedStateAction<S, E>,
    ) -> &mut dyn StateConfigurer<S, E> {
        self.add_incomplete(
            self.current_parent(),
            state,
            None,
            None,
            Some(vec![action]),
            None,
        );
        self
    }

    fn state_do_with_error(
        &mut self,
        state: S,
        action: Arc<dyn StateMachineAction<S, E>>,
        error_action: Option<Arc<dyn StateMachineAction<S, E>>>,
    ) -> &mut dyn StateConfigurer<S, E> {
        let state_actions = if let Some(error_action) = error_action {
            Actions::error_calling_action(action, error_action)
        } else {
            action
        };
        self.state_with_actions(state, vec![state_actions])
    }

    fn state_with_entry_and_exit_functions(
        &mut self,
        state: S,
        entry_actions: Vec<Arc<dyn StateMachineAction<S, E>>>,
        exit_actions: Vec<Arc<dyn StateMachineAction<S, E>>>,
    ) -> &mut dyn StateConfigurer<S, E> {
        let r_entry_actions = if !entry_actions.is_empty() {
            Some(
                entry_actions
                    .into_iter()
                    .map(|action| Actions::from(action))
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        };
        let r_exit_actions = if !exit_actions.is_empty() {
            Some(
                exit_actions
                    .into_iter()
                    .map(|action| Actions::from(action))
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        };

        self.add_incomplete(
            self.current_parent(),
            state,
            None,
            r_entry_actions,
            r_exit_actions,
            None,
        );
        self
    }

    fn state_with_entry_and_exit_function(
        &mut self,
        state: S,
        entry_action: Arc<dyn StateMachineAction<S, E>>,
        exit_action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn StateConfigurer<S, E> {
        self.state_with_entry_and_exit_functions(state, vec![entry_action], vec![exit_action])
    }

    fn state_entry(
        &mut self,
        state: S,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn StateConfigurer<S, E> {
        self.state_with_entry_and_exit_functions(state, vec![action], Vec::new())
    }

    fn state_entry_with_error(
        &mut self,
        state: S,
        action: Arc<dyn StateMachineAction<S, E>>,
        error_action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn StateConfigurer<S, E> {
        let entry_actions = vec![Actions::error_calling_action(action, error_action)];
        self.state_with_entry_and_exit_functions(state, entry_actions, Vec::new())
    }

    fn state_exit(
        &mut self,
        state: S,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn StateConfigurer<S, E> {
        self.state_with_entry_and_exit_functions(state, Vec::new(), vec![action])
    }

    fn state_exity_with_error(
        &mut self,
        state: S,
        action: Arc<dyn StateMachineAction<S, E>>,
        error_action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn StateConfigurer<S, E> {
        let exit_actions = vec![Actions::error_calling_action(action, error_action)];
        self.state_with_entry_and_exit_functions(state, Vec::new(), exit_actions)
    }

    fn state_deferred(&mut self, state: S, deferred: Vec<E>) -> &mut dyn StateConfigurer<S, E> {
        self.add_incomplete(
            self.current_parent(),
            state,
            Some(deferred),
            None,
            None,
            None,
        );
        self
    }

    fn states(&mut self, states: Vec<S>) -> &mut dyn StateConfigurer<S, E> {
        for state in states {
            self.state(state);
        }
        self
    }

    fn choice(&mut self, choice: S) -> &mut dyn StateConfigurer<S, E> {
        self.state(choice.clone());
        self.shared.borrow_mut().choices.push(choice);
        self
    }

    fn junction(&mut self, junction: S) -> &mut dyn StateConfigurer<S, E> {
        self.state(junction.clone());
        self.shared.borrow_mut().junctions.push(junction);
        self
    }

    fn fork(&mut self, fork: S) -> &mut dyn StateConfigurer<S, E> {
        self.state(fork.clone());
        self.shared.borrow_mut().forks.push(fork);
        self
    }

    fn join(&mut self, join: S) -> &mut dyn StateConfigurer<S, E> {
        self.state(join.clone());
        self.shared.borrow_mut().joins.push(join);
        self
    }

    fn history(&mut self, history: S, history_type: History) -> &mut dyn StateConfigurer<S, E> {
        {
            let mut data = self.shared.borrow_mut();
            data.history = Some(history.clone());
            data.history_type = Some(history_type);
        }
        self.state(history);
        self
    }

    fn entry(&mut self, entry: S) -> &mut dyn StateConfigurer<S, E> {
        self.state(entry.clone());
        self.shared.borrow_mut().entrys.push(entry);
        self
    }

    fn exit(&mut self, exit: S) -> &mut dyn StateConfigurer<S, E> {
        self.state(exit.clone());
        self.shared.borrow_mut().exits.push(exit);
        self
    }
}

impl<S, E> DefaultStateConfigurer<S, E>
where
    S: Eq + Hash,
    S: Send + Sync + 'static,
    S: Clone,
    E: Send + Sync + 'static,
    E: Clone,
{
    fn current_parent(&self) -> Option<S> {
        self.shared.borrow().parent.clone()
    }

    pub fn add_incomplete(
        &mut self,
        parent: Option<S>,
        state: S,
        deferred: Option<Vec<E>>,
        entry_actions: Option<Vec<BoxedStateAction<S, E>>>,
        exit_actions: Option<Vec<BoxedStateAction<S, E>>>,
        state_actions: Option<Vec<BoxedStateAction<S, E>>>,
    ) -> &mut Self {
        let mut data = self.shared.borrow_mut();
        let region = data.region.clone();
        let state_data = data
            .incomplete
            .entry(state.clone())
            .or_insert(StateData::with_actions(
                parent.as_ref().map(|s| Arc::new(s.clone()) as Arc<dyn Any>),
                Some(region.clone()),
                state,
                deferred.clone(),
                entry_actions.clone(),
                exit_actions.clone(),
            ));

        if state_data.parent().is_none() {
            if let Some(parent) = parent.map(|s| Arc::new(s) as Arc<dyn Any>) {
                state_data.set_parent(parent);
            }
        }

        if state_data.region().is_none() {
            state_data.set_region(region.as_str());
        }

        if state_data.deferred().is_none() {
            if let Some(deferred) = deferred {
                state_data.set_deferred(deferred);
            }
        }

        if state_data.entry_actions().is_none() {
            if let Some(entry_actions) = entry_actions {
                state_data.set_entry_actions(entry_actions);
            }
        }

        if state_data.exit_actions().is_none() {
            if let Some(exit_actions) = exit_actions {
                state_data.set_exit_actions(exit_actions);
            }
        }

        if state_data.state_actions().is_none() {
            if let Some(state_actions) = state_actions {
                state_data.set_state_actions(state_actions);
            }
        }

        drop(data);
        self
    }
}

impl<S, E> Default for DefaultStateConfigurer<S, E>
where
    S: Eq + Hash,
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn default() -> Self {
        Self {
            shared: Rc::new(RefCell::new(DefaultStateConfigurerData {
                parent: None,
                region: Uuid::new_v4().to_string(),
                incomplete: HashMap::new(),
                initial_state: None,
                initial_action: None,
                ends: Vec::new(),
                history: None,
                history_type: None,
                choices: Vec::new(),
                junctions: Vec::new(),
                forks: Vec::new(),
                joins: Vec::new(),
                exits: Vec::new(),
                entrys: Vec::new(),
                submachines: HashMap::new(),
                submachine_factories: HashMap::new(),
            })),

            base: Default::default(),
        }
    }
}

impl<S, E> Clone for DefaultStateConfigurer<S, E>
where
    S: Eq + Hash + Clone + 'static,
    E: Clone + 'static,
{
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
            base: self.base.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{
        builders::state_machine_state_builder::StateMachineStateBuilder,
        configurers::{
            default_state_configurer::DefaultStateConfigurer, state_configurer::StateConfigurer,
        },
    };

    #[test]
    fn parent_scope_is_applied_to_added_states() {
        let mut configurer = DefaultStateConfigurer::<i32, i32>::default();
        configurer.parent(1).initial(2).state(3);

        let mut builder = StateMachineStateBuilder::new(true);
        crate::config::common::configurer_adapter::ConfigurerAdapterExt::configure(
            &mut configurer,
            &mut builder,
        )
        .unwrap();

        let states = crate::config::common::builder::Builder::build(&mut builder).unwrap();
        let mut states = states.state_data().iter().collect::<Vec<_>>();
        states.sort_by_key(|state| *state.state());

        assert_eq!(states.len(), 2);
        assert_eq!(states[0].state(), &2);
        assert_eq!(
            states[0]
                .parent()
                .and_then(|parent| parent.downcast_ref::<i32>()),
            Some(&1)
        );
        assert!(states[0].is_initial());
        assert_eq!(states[1].state(), &3);
        assert_eq!(
            states[1]
                .parent()
                .and_then(|parent| parent.downcast_ref::<i32>()),
            Some(&1)
        );
    }
}
