use std::any::Any;

use next_web_core::{clone_trait_object, error::BoxError, DynClone};

use crate::config::configurers::{
    configuration_configurer::ConfigurationConfigurer,
    distributed_state_machine_configurer::DistributedStateMachineConfigurer,
    monitoring_configurer::MonitoringConfigurer, persistence_configurer::PersistenceConfigurer,
    security_configurer::StateMachineSecurityConfigurer,
    verifier_configurer::StateMachineModelVerifierConfigurer,
};

/// Configurer interface exposing generic config.
pub trait StateMachineConfigurationConfigurer<S, E>
where
    Self: DynClone,
{
    /// Gets a configurer for generic config.
    fn with_configuration(&mut self) -> Result<Box<dyn ConfigurationConfigurer<S, E>>, BoxError>;

    /// Gets a configurer for distributed state machine config.
    fn with_distributed(
        &mut self,
    ) -> Result<Box<dyn DistributedStateMachineConfigurer<S, E>>, BoxError>;

    /// Gets a configurer for securing state machine.
    fn with_security(&mut self) -> Result<Box<dyn StateMachineSecurityConfigurer<S, E>>, BoxError>;

    /// Gets a configurer for state machine model verifier.
    fn with_verifier(
        &mut self,
    ) -> Result<Box<dyn StateMachineModelVerifierConfigurer<S, E>>, BoxError>;

    ///  Gets a configurer for state machine monitoring.
    fn with_monitoring(&mut self) -> Result<Box<dyn MonitoringConfigurer<S, E>>, BoxError>;

    /// Gets a configurer for state machine persistence.
    fn with_persistence(
        &mut self,
    ) -> Result<Box<dyn PersistenceConfigurer<S, E, Box<dyn Any>>>, BoxError>;
}

clone_trait_object!(<S, E> StateMachineConfigurationConfigurer<S, E>);
