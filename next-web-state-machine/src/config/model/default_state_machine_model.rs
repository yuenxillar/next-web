use crate::config::model::{
    configuration_data::ConfigurationData, state_machine_model::StateMachineModel,
    states_data::StatesData, transitions_data::TransitionsData,
};

/// The default model of a machine.
///
/// A model carries the configuration, the states and the transitions, which is
/// everything a factory needs to build a running machine.
#[derive(Clone)]
pub struct DefaultStateMachineModel<S, E> {
    pub(crate) configuration: ConfigurationData<S, E>,
    pub(crate) states: Option<StatesData<S, E>>,
    pub(crate) transitions: Option<TransitionsData<S, E>>,
}

impl<S, E> DefaultStateMachineModel<S, E> {
    /// Creates a new model.
    ///
    /// # Arguments
    /// * `configuration` - the configuration of the machine
    /// * `states` - the states of the machine
    /// * `transitions` - the transitions of the machine
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

    /// Gets the mutable configuration data of the model.
    pub fn get_configuration_data(&mut self) -> &mut ConfigurationData<S, E> {
        &mut self.configuration
    }

    /// Gets the mutable states data of the model.
    pub fn get_states_data(&mut self) -> Option<&mut StatesData<S, E>> {
        self.states.as_mut()
    }

    /// Gets the mutable transitions data of the model.
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
