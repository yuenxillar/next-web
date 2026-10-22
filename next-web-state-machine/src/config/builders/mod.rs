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

pub use state_machine_config_builder::StateMachineConfigBuilder;
pub use state_machine_configuration_builder::StateMachineConfigurationBuilder;
pub use state_machine_configuration_configurer::StateMachineConfigurationConfigurer;
pub use state_machine_configurer::StateMachineConfigurer;
pub use state_machine_model_builder::StateMachineModelBuilder;
pub use state_machine_model_configurer::StateMachineModelConfigurer;
pub use state_machine_state_builder::StateMachineStateBuilder;
pub use state_machine_state_configurer::StateMachineStateConfigurer;
pub use state_machine_transition_builder::StateMachineTransitionBuilder;
pub use state_machine_transition_configurer::StateMachineTransitionConfigurer;
