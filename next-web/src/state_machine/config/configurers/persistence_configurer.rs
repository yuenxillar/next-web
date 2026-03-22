use std::sync::Arc;

use crate::state_machine::{
    config::{
        builders::state_machine_configuration_configurer::StateMachineConfigurationConfigurer,
        configurer_builder::ConfigurerBuilder,
    },
    persist::state_machine_runtime_persister::StateMachineRuntimePersister,
};

/// Base PersistenceConfigurer interface for configuring state machine persistence.
pub trait PersistenceConfigurer<S, E, V>
where
    Self: ConfigurerBuilder<Box<dyn StateMachineConfigurationConfigurer<S, E>>>,
{
    /// Specify a state machine runtime persister.
    fn runtime_persister(
        &mut self,
        persister: Arc<dyn StateMachineRuntimePersister<S, E, V>>,
    ) -> &mut dyn PersistenceConfigurer<S, E, V>;
}
