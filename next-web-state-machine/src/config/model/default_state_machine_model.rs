use crate::config::model::{
    configuration_data::ConfigurationData, state_machine_model::StateMachineModel,
    states_data::StatesData, transitions_data::TransitionsData,
};

#[derive(Clone)]
pub struct DefaultStateMachineModel<S, E> {
    pub(crate) configuration: ConfigurationData<S, E>,
    pub(crate) states: Option<StatesData<S, E>>,
    pub(crate) transitions: Option<TransitionsData<S, E>>,
}

impl<S, E> DefaultStateMachineModel<S, E> {
    pub fn new(
        configuration: ConfigurationData<S, E>,
        states: Option<StatesData<S, E>>,
        transitions: Option<TransitionsData<S, E>>,
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

    pub fn get_states_data(&mut self) -> Option<&mut StatesData<S, E>> {
        self.states.as_mut()
    }

    pub fn get_transitions_data(&mut self) -> Option<&mut TransitionsData<S, E>> {
        self.transitions.as_mut()
    }
}

impl<S, E> StateMachineModel<S, E> for DefaultStateMachineModel<S, E>
where
    S: Send + Sync,
    E: Send + Sync,
{
    /// Gets the configuration config data.
    fn get_configuration_data(&self) -> &ConfigurationData<S, E> {
        &self.configuration
    }

    /// Gets the states config data.
    fn get_states_data(&self) -> Option<&StatesData<S, E>> {
        self.states.as_ref()
    }

    /// Gets the transitions config data.
    fn get_transitions_data(&self) -> Option<&TransitionsData<S, E>> {
        self.transitions.as_ref()
    }
}
