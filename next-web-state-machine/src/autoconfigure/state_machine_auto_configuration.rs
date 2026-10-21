use std::{fmt::Debug, hash::Hash, sync::Arc};

use next_web_core::error::BoxError;

use crate::{
    StateMachine,
    config::{
        builders::{
            state_machine_config_builder::StateMachineConfigBuilder,
            state_machine_configurer::StateMachineConfigurer,
        },
        common::base_configured_builder::BaseConfiguredBuilderExtOwn,
        default_state_machine_factory::DefaultStateMachineFactory,
        state_machine_configurer_adapter::StateMachineConfigurerAdapter,
        state_machine_factory::StateMachineFactory,
    },
};

/// Builds a machine from a configurer with the default factory.
pub struct StateMachineAutoConfiguration;

impl StateMachineAutoConfiguration {
    /// Builds a machine from the given configurer.
    ///
    /// # Arguments
    /// * `state_machine_configurer` - the configurer of the machine
    pub fn build<T, S, E>(
        state_machine_configurer: T,
    ) -> Result<Arc<dyn StateMachine<S, E>>, BoxError>
    where
        S: Send + Sync + 'static,
        S: Eq + Hash,
        S: Clone + Debug,
        E: Send + Sync + 'static,
        E: Clone,
        E: Eq + Hash,
        T: StateMachineConfigurer<S, E>,
        T: 'static,
        T: Clone,
    {
        let mut configurer =
            StateMachineConfigurerAdapter::with_state_machine_configurer(state_machine_configurer);

        let mut builder = StateMachineConfigBuilder::<S, E>::default();
        builder.apply(&mut configurer)?;

        let state_machine_factory = DefaultStateMachineFactory::create(&mut builder)?;
        let _ = builder.get_or_build();

        state_machine_factory.get_state_machine()
    }
}
