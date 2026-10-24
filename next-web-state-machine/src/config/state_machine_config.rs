use crate::config::{
    builders::state_machine_model_builder::ModelData,
    model::{
        configuration_data::ConfigurationData, states_data::StatesData,
        transitions_data::TransitionsData,
    },
};

/// Generic struct keeping related configs together.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
#[derive(Clone)]
pub struct StateMachineConfig<S, E> {
    state_machine_configuration_config: ConfigurationData<S, E>,
    transitions: TransitionsData<S, E>,
    states: StatesData<S, E>,
    model: Option<ModelData<S, E>>,
}

impl<S, E> StateMachineConfig<S, E> {
    /// Creates a new state machine config.
    ///
    /// # Arguments
    /// * `state_machine_configuration_config` - the state machine configuration config
    /// * `transitions` - the transitions
    /// * `states` - the states
    pub fn new(
        state_machine_configuration_config: ConfigurationData<S, E>,
        transitions: TransitionsData<S, E>,
        states: StatesData<S, E>,
        model: Option<ModelData<S, E>>,
    ) -> Self {
        Self {
            state_machine_configuration_config,
            transitions,
            states,
            model,
        }
    }

    /// Creates a new state machine config with model.
    ///
    /// # Arguments
    /// * `state_machine_configuration_config` - the state machine configuration config
    /// * `transitions` - the transitions
    /// * `states` - the states
    /// * `model` - the model
    pub fn with_model(
        state_machine_configuration_config: ConfigurationData<S, E>,
        transitions: TransitionsData<S, E>,
        states: StatesData<S, E>,
        model: ModelData<S, E>,
    ) -> Self {
        Self {
            state_machine_configuration_config,
            transitions,
            states,
            model: Some(model),
        }
    }

    /// Returns a reference to the state machine configuration config.
    pub fn state_machine_configuration_config(&self) -> &ConfigurationData<S, E> {
        &self.state_machine_configuration_config
    }

    /// Returns a reference to the transitions.
    pub fn transitions(&self) -> &TransitionsData<S, E> {
        &self.transitions
    }

    /// Returns a reference to the states.
    pub fn states(&self) -> &StatesData<S, E> {
        &self.states
    }

    /// Returns an option reference to the model.
    pub fn model(&self) -> Option<&ModelData<S, E>> {
        self.model.as_ref()
    }
}
