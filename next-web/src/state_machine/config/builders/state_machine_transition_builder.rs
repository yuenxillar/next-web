use std::{collections::HashMap, hash::Hash};

use next_web_core::error::BoxError;

use crate::state_machine::config::{
    builders::state_machine_transition_configurer::StateMachineTransitionConfigurer,
    common::{base_configured_builder::BaseConfiguredBuilder, builder::Builder},
    configurers::{
        choice_transition_configurer::ChoiceTransitionConfigurer,
        entry_transition_configurer::EntryTransitionConfigurer,
        exit_transition_configurer::ExitTransitionConfigurer,
        external_transition_configurer::ExternalTransitionConfigurer,
        fork_transition_configurer::ForkTransitionConfigurer,
        history_transition_configurer::HistoryTransitionConfigurer,
        internal_transition_configurer::InternalTransitionConfigurer,
        join_transition_configurer::JoinTransitionConfigurer,
        junction_transition_configurer::JunctionTransitionConfigurer,
        local_transition_configurer::LocalTransitionConfigurer,
    },
    model::{
        choice_data::ChoiceData, entry_data::EntryData, exit_data::ExitData,
        history_data::HistoryData, junction_data::JunctionData, transition_data::TransitionData,
        transitions_data::TransitionsData,
    },
};

#[derive(Clone)]
pub struct StateMachineTransitionBuilder<S, E>
where
    S: Eq + Hash,
{
    transition_data: Vec<TransitionData<S, E>>,
    choices: HashMap<S, ChoiceData<S, E>>,
    junctions: HashMap<S, JunctionData<S, E>>,
    forks: HashMap<S, Vec<S>>,
    joins: HashMap<S, Vec<S>>,

    entry_data: Vec<EntryData<S, E>>,
    exit_data: Vec<ExitData<S, E>>,
    history_data: Vec<HistoryData<S, E>>,

    base: BaseConfiguredBuilder<
        TransitionsData<S, E>,
        Box<dyn StateMachineTransitionConfigurer<S, E>>,
        StateMachineTransitionBuilder<S, E>,
    >,
}

impl<S, E> StateMachineTransitionBuilder<S, E>
where
    S: Eq + Hash,
{
    pub fn new(allow_configurers_of_same_type: bool) -> Self {
        let base = BaseConfiguredBuilder::with_allow_configurers_of_same_type(
            allow_configurers_of_same_type,
        );

        Self {
            transition_data: Default::default(),
            choices: Default::default(),
            junctions: Default::default(),
            forks: Default::default(),
            joins: Default::default(),
            entry_data: Default::default(),
            exit_data: Default::default(),
            history_data: Default::default(),
            base,
        }
    }
}

impl<S, E> Builder<TransitionsData<S, E>> for StateMachineTransitionBuilder<S, E>
where
    S: Eq + Hash,
{
    fn build(&mut self) -> Result<TransitionsData<S, E>, BoxError> {
        todo!()
    }
}

impl<S, E> StateMachineTransitionConfigurer<S, E> for StateMachineTransitionBuilder<S, E>
where
    S: Eq + Hash,
    S: Clone,
    E: Clone,
{
    fn with_external(&mut self) -> Result<Box<dyn ExternalTransitionConfigurer<S, E>>, BoxError> {
        // DefaultExternalTransitionConfigurer::default();
        todo!()
    }

    fn with_internal(&mut self) -> Result<Box<dyn InternalTransitionConfigurer<S, E>>, BoxError> {
        todo!()
    }

    fn with_local(&mut self) -> Result<Box<dyn LocalTransitionConfigurer<S, E>>, BoxError> {
        todo!()
    }

    fn with_choice(&mut self) -> Result<Box<dyn ChoiceTransitionConfigurer<S, E>>, BoxError> {
        todo!()
    }

    fn with_junction(&mut self) -> Result<Box<dyn JunctionTransitionConfigurer<S, E>>, BoxError> {
        todo!()
    }

    fn with_fork(&mut self) -> Result<Box<dyn ForkTransitionConfigurer<S, E>>, BoxError> {
        todo!()
    }

    fn with_join(&mut self) -> Result<Box<dyn JoinTransitionConfigurer<S, E>>, BoxError> {
        todo!()
    }

    fn with_entry(&mut self) -> Result<Box<dyn EntryTransitionConfigurer<S, E>>, BoxError> {
        todo!()
    }

    fn with_exit(&mut self) -> Result<Box<dyn ExitTransitionConfigurer<S, E>>, BoxError> {
        todo!()
    }

    fn with_history(&mut self) -> Result<Box<dyn HistoryTransitionConfigurer<S, E>>, BoxError> {
        todo!()
    }
}
