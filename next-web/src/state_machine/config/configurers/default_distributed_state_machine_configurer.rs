use std::sync::Arc;

use next_web_core::error::BoxError;

use crate::state_machine::{
    config::{
        builders::{
            state_machine_configuration_builder::StateMachineConfigurationBuilder,
            state_machine_configuration_configurer::StateMachineConfigurationConfigurer,
        },
        common::configurer_adapter::{ConfigurerAdapter, ConfigurerAdapterExt},
        configurer_builder::ConfigurerBuilder,
        configurers::distributed_state_machine_configurer::DistributedStateMachineConfigurer,
        model::configuration_data::ConfigurationData,
    },
    ensemble::state_machine_ensemble::StateMachineEnsemble,
};

/// Default implementation of a `DistributedStateMachineConfigurer`.
pub struct DefaultDistributedStateMachineConfigurer<S, E> {
    ensemble: Option<Arc<dyn StateMachineEnsemble<S, E>>>,

    pub(crate) base: ConfigurerAdapter<
        ConfigurationData<S, E>,
        Box<dyn StateMachineConfigurationConfigurer<S, E>>,
        StateMachineConfigurationBuilder<S, E>,
    >,
}

impl<S, E> DefaultDistributedStateMachineConfigurer<S, E> {
    /// Creates a new distributed state machine configurer.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<S, E> ConfigurerBuilder<Box<dyn StateMachineConfigurationConfigurer<S, E>>>
    for DefaultDistributedStateMachineConfigurer<S, E>
{
    fn and(self) -> Box<dyn StateMachineConfigurationConfigurer<S, E>> {
        todo!()
    }
}

impl<S, E> DistributedStateMachineConfigurer<S, E>
    for DefaultDistributedStateMachineConfigurer<S, E>
{
    fn ensemble(
        &mut self,
        ensemble: Arc<dyn StateMachineEnsemble<S, E>>,
    ) -> &mut dyn DistributedStateMachineConfigurer<S, E> {
        self.ensemble = Some(ensemble);
        self
    }
}

impl<S, E> ConfigurerAdapterExt<ConfigurationData<S, E>, StateMachineConfigurationBuilder<S, E>>
    for DefaultDistributedStateMachineConfigurer<S, E>
{
    fn configure(
        &mut self,
        builder: &mut StateMachineConfigurationBuilder<S, E>,
    ) -> Result<(), BoxError> {
        if let Some(ensemble) = &self.ensemble {
            // In a real implementation, we would set the ensemble on the builder
            // For now, we'll extend the ConfigurationData to include ensemble
            builder.set_state_machine_ensemble(ensemble.clone());
        }
        Ok(())
    }
}

impl<S, E> Default for DefaultDistributedStateMachineConfigurer<S, E> {
    fn default() -> Self {
        Self {
            ensemble: None,
            base: Default::default(),
        }
    }
}
