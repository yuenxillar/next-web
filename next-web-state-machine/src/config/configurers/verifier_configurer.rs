use std::sync::Arc;

use crate::config::{
    builders::state_machine_configuration_configurer::StateMachineConfigurationConfigurer,
    configurer_builder::ConfigurerBuilder,
    model::verifier::state_machine_model_verifier::StateMachineModelVerifier,
};

/// Base ConfigConfigurer interface for configuring state machine model verifier.
pub trait StateMachineModelVerifierConfigurer<S, E>
where
    Self: ConfigurerBuilder<Box<dyn StateMachineConfigurationConfigurer<S, E>>>,
{
    /// Specify if verifier is enabled. On default verifier is enabled.
    fn enabled(&mut self, enabled: bool) -> &mut dyn StateMachineModelVerifierConfigurer<S, E>;

    /// Specify a  StateMachineEnsemble.
    fn verifier(
        &mut self,
        verifier: Arc<dyn StateMachineModelVerifier<S, E>>,
    ) -> &mut dyn StateMachineModelVerifierConfigurer<S, E>;
}
