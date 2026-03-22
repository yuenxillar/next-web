use crate::state_machine::config::model::{
    configuration_data::ConfigurationData, state_machine_model::StateMachineModel,
    states_data::StatesData, transitions_data::TransitionsData,
};

#[derive(Clone)]
pub struct DefaultStateMachineModel<S, E> {
    pub(crate) configuration: ConfigurationData<S, E>,
    pub(crate) states: StatesData<S, E>,
    pub(crate) transitions: TransitionsData<S, E>,
}

impl<S, E> DefaultStateMachineModel<S, E> {
    pub fn new(
        configuration: ConfigurationData<S, E>,
        states: StatesData<S, E>,
        transitions: TransitionsData<S, E>,
    ) -> Self {
        DefaultStateMachineModel {
            configuration,
            states,
            transitions,
        }
    }

    // 获取可变引用
    pub fn get_configuration_data(&mut self) -> &mut ConfigurationData<S, E> {
        &mut self.configuration
    }

    pub fn get_states_data(&mut self) -> &mut StatesData<S, E> {
        &mut self.states
    }

    pub fn get_transitions_data(&mut self) -> &mut TransitionsData<S, E> {
        &mut self.transitions
    }
}

impl<S, E> StateMachineModel<S, E> for DefaultStateMachineModel<S, E>
where
    S: Send + Sync,
    E: Send + Sync,
{
    /// Gets the configuration config data.
    fn get_configuration_data(&self) -> Option<&ConfigurationData<S, E>> {
        Some(&self.configuration)
    }

    /// Gets the states config data.
    fn get_states_data(&self) -> &StatesData<S, E> {
        &self.states
    }

    /// Gets the transitions config data.
    fn get_transitions_data(&self) -> &TransitionsData<S, E> {
        &self.transitions
    }
}
