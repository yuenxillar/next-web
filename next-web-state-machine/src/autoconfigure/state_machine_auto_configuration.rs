use std::{fmt::Debug, hash::Hash, sync::Arc};

use next_web_core::error::BoxError;

use crate::{
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
    StateMachine,
};

pub struct StateMachineAutoConfiguration;

impl StateMachineAutoConfiguration {
    /// 123
    pub fn build<T, S, E>(
        state_machine_configurer: T,
    ) -> Result<Arc<dyn StateMachine<S, E>>, BoxError>
    where
        S: Send + Sync + 'static,
        S: Eq + Hash,
        S: Clone + Debug,
        E: Send + Sync + 'static,
        E: Clone,
        E: Eq,

        T: StateMachineConfigurer<S, E>,
        T: 'static,
        T: Clone,
    {
        let mut configurer =
            StateMachineConfigurerAdapter::with_state_machine_configurer(state_machine_configurer);

        let mut builder = StateMachineConfigBuilder::<S, E>::default();
        builder.apply(&mut configurer)?;

        let state_machine_factory = DefaultStateMachineFactory::create(&mut builder);
        builder.get_or_build();

        state_machine_factory.get_state_machine()
    }
}
