use std::{any::Any, sync::Arc};

use next_web_core::error::BoxError;

use crate::state_machine::{
    config::{
        builders::{
            state_machine_configuration_builder::StateMachineConfigurationBuilder,
            state_machine_configuration_configurer::StateMachineConfigurationConfigurer,
        },
        common::configurer_adapter::{ConfigurerAdapter, ConfigurerAdapterExt},
        configurer_builder::ConfigurerBuilder,
        configurers::persistence_configurer::PersistenceConfigurer,
        model::configuration_data::ConfigurationData,
    },
    persist::state_machine_runtime_persister::StateMachineRuntimePersister,
};

/// Default implementation of a `VerifierConfigurer`.
pub struct DefaultPersistenceConfigurer<S, E> {
    persister: Option<Arc<dyn StateMachineRuntimePersister<S, E, Box<dyn Any>>>>,

    pub(crate) base: ConfigurerAdapter<
        ConfigurationData<S, E>,
        Box<dyn StateMachineConfigurationConfigurer<S, E>>,
        StateMachineConfigurationBuilder<S, E>,
    >,
}

impl<S, E> DefaultPersistenceConfigurer<S, E> {
    /// Creates a new verifier configurer.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<S, E> ConfigurerBuilder<Box<dyn StateMachineConfigurationConfigurer<S, E>>>
    for DefaultPersistenceConfigurer<S, E>
{
    fn and(self) -> Box<dyn StateMachineConfigurationConfigurer<S, E>> {
        todo!()
    }
}

impl<S, E> PersistenceConfigurer<S, E, Box<dyn Any>> for DefaultPersistenceConfigurer<S, E> {
    fn runtime_persister(
        &mut self,
        persister: Arc<dyn StateMachineRuntimePersister<S, E, Box<dyn Any>>>,
    ) -> &mut dyn PersistenceConfigurer<S, E, Box<dyn Any>> {
        self.persister = Some(persister);

        self
    }
}

impl<S, E> ConfigurerAdapterExt<ConfigurationData<S, E>, StateMachineConfigurationBuilder<S, E>>
    for DefaultPersistenceConfigurer<S, E>
{
    fn configure(
        &mut self,
        builder: &mut StateMachineConfigurationBuilder<S, E>,
    ) -> Result<(), BoxError> {
        self.persister
            .as_ref()
            .map(Clone::clone)
            .map(|persister| builder.set_state_machine_runtime_persister(persister));

        Ok(())
    }
}

impl<S, E> Default for DefaultPersistenceConfigurer<S, E> {
    fn default() -> Self {
        Self {
            persister: None,

            base: Default::default(),
        }
    }
}
