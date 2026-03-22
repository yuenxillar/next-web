use std::hash::Hash;

use next_web_core::error::BoxError;

use crate::state_machine::config::{
    builders::state_machine_state_configurer::StateMachineStateConfigurer,
    common::{
        base_configured_builder::{BaseConfiguredBuilder, BaseConfiguredBuilderExt},
        builder::Builder,
    },
    configurers::{
        default_state_configurer::DefaultStateConfigurer, state_configurer::StateConfigurer,
    },
    model::{state_data::StateData, states_data::StatesData},
};

#[derive(Clone)]
pub struct StateMachineStateBuilder<S, E> {
    state_datas: Vec<StateData<S, E>>,

    base: BaseConfiguredBuilder<
        StatesData<S, E>,
        Box<dyn StateMachineStateConfigurer<S, E>>,
        StateMachineStateBuilder<S, E>,
    >,
}

impl<S, E> StateMachineStateBuilder<S, E> {
    pub fn new(allow_configurers_of_same_type: bool) -> Self {
        Self {
            state_datas: Vec::new(),

            base: BaseConfiguredBuilder::with_allow_configurers_of_same_type(
                allow_configurers_of_same_type,
            ),
        }
    }

    pub fn add_state_data(&mut self, state_data: Vec<StateData<S, E>>) {
        self.state_datas.extend(state_data);
    }
}

impl<S, E> BaseConfiguredBuilderExt<StatesData<S, E>> for StateMachineStateBuilder<S, E>
where
    S: Clone,
    E: Clone,
{
    fn perform_build(&mut self) -> Result<StatesData<S, E>, BoxError> {
        Ok(StatesData::new(self.state_datas.clone()))
    }
}

impl<S, E> Builder<StatesData<S, E>> for StateMachineStateBuilder<S, E> {
    fn build(&mut self) -> Result<StatesData<S, E>, BoxError> {
        todo!()
    }
}

impl<S, E> StateMachineStateConfigurer<S, E> for StateMachineStateBuilder<S, E>
where
    S: Eq + Hash,
    S: Send + Sync,
    S: Clone,
    E: Send + Sync,
    E: Clone,
{
    fn with_states(&mut self) -> Result<Box<dyn StateConfigurer<S, E>>, BoxError> {
        let mut state_configurer = DefaultStateConfigurer::default();
        self.base.apply_adapter(&mut state_configurer.base)?;

        Ok(Box::new(state_configurer))
    }
}

impl<S, E> Default for StateMachineStateBuilder<S, E>
where
    S: Clone,
    E: Clone,
{
    fn default() -> Self {
        Self {
            state_datas: Vec::new(),

            base: Default::default(),
        }
    }
}
