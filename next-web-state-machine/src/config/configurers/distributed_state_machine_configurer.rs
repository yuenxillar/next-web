use std::sync::Arc;

use crate::{
    config::{
        builders::state_machine_configuration_configurer::StateMachineConfigurationConfigurer,
        configurer_builder::ConfigurerBuilder,
    },
    ensemble::state_machine_ensemble::StateMachineEnsemble,
};

/// Base DistributedStateMachineConfigurer interface for configuring distributed state machine.
pub trait DistributedStateMachineConfigurer<S, E>
where
    Self: ConfigurerBuilder<Box<dyn StateMachineConfigurationConfigurer<S, E>>>,
{
    /// Specify a  StateMachineEnsemble.
    fn ensemble(
        &mut self,
        ensemble: Arc<dyn StateMachineEnsemble<S, E>>,
    ) -> &mut dyn DistributedStateMachineConfigurer<S, E>;
}
