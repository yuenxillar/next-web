use crate::config::model::{
    configuration_data::ConfigurationData, states_data::StatesData,
    transitions_data::TransitionsData,
};

/// Base abstract SPI class for state machine configuration.
pub trait StateMachineModel<S, E>
where
    S: Send + Sync,
    E: Send + Sync,
{
    /// Gets the configuration config data.
    fn get_configuration_data(&self) -> &ConfigurationData<S, E>;

    /// Gets the states config data.
    fn get_states_data(&self) -> Option<&StatesData<S, E>>;

    /// Gets the transitions config data.
    fn get_transitions_data(&self) -> Option<&TransitionsData<S, E>>;
}
