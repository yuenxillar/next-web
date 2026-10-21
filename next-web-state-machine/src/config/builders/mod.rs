//! Fluent builders which assemble a machine from a model.
/// Builder of the whole machine configuration.
pub mod state_machine_config_builder;
/// Builder of the generic configuration.
pub mod state_machine_configuration_builder;
/// Configurer interface exposing the generic configuration.
pub mod state_machine_configuration_configurer;
/// Builder of the machine configuration.
pub mod state_machine_configurer;
/// Builder of the model.
pub mod state_machine_model_builder;
/// Configurer interface exposing the model.
pub mod state_machine_model_configurer;
/// Builder of the states.
pub mod state_machine_state_builder;
/// Configurer interface exposing the states.
pub mod state_machine_state_configurer;
/// Builder of the transitions.
pub mod state_machine_transition_builder;
/// Configurer interface exposing the transitions.
pub mod state_machine_transition_configurer;
