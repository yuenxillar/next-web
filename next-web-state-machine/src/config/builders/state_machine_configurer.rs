use std::hash::Hash;

use next_web_core::error::BoxError;

use crate::config::{
    builders::{
        state_machine_config_builder::StateMachineConfigBuilder,
        state_machine_configuration_builder::StateMachineConfigurationBuilder,
        state_machine_configuration_configurer::StateMachineConfigurationConfigurer,
        state_machine_model_builder::StateMachineModelBuilder,
        state_machine_model_configurer::StateMachineModelConfigurer,
        state_machine_state_builder::StateMachineStateBuilder,
        state_machine_state_configurer::StateMachineStateConfigurer,
        state_machine_transition_builder::StateMachineTransitionBuilder,
        state_machine_transition_configurer::StateMachineTransitionConfigurer,
    },
    common::configurer::Configurer,
    state_machine_config::StateMachineConfig,
};

pub trait StateMachineConfigurer<
    S,
    E,
    T1 = StateMachineConfigurationBuilder<S, E>,
    T2 = StateMachineModelBuilder<S, E>,
    T3 = StateMachineStateBuilder<S, E>,
    T4 = StateMachineTransitionBuilder<S, E>,
> where
    S: Send + Sync + 'static,
    S: Clone,
    S: Eq + Hash,
    E: Send + Sync + 'static,
    E: Clone,
    E: Eq,
    T1: StateMachineConfigurationConfigurer<S, E>,
    T2: StateMachineModelConfigurer<S, E>,
    T3: StateMachineStateConfigurer<S, E>,
    T4: StateMachineTransitionConfigurer<S, E>,
    Self: Configurer<StateMachineConfig<S, E>, StateMachineConfigBuilder<S, E>>,
{
    /// Callback for StateMachineConfigurationConfigurer.
    fn config_configure(&mut self, config: &mut T1) -> Result<(), BoxError>;

    /// Callback for StateMachineModelConfigurer.
    fn model_configure(&mut self, model: &mut T2) -> Result<(), BoxError>;

    /// Callback for StateMachineStateConfigurer.
    fn state_configure(&mut self, states: &mut T3) -> Result<(), BoxError>;

    /// Callback for StateMachineTransitionConfigurer.
    fn transition_configure(&mut self, transitions: &mut T4) -> Result<(), BoxError>;
}
