use std::sync::Arc;

use crate::state_machine::config::{
    builders::state_machine_model_configurer::StateMachineModelConfigurer,
    configurer_builder::ConfigurerBuilder,
    model::state_machine_model_factory::StateMachineModelFactory,
};

/// Base ModelConfigurer interface for configuring state machine model.
pub trait ModelConfigurer<S, E>
where
    Self: ConfigurerBuilder<Box<dyn StateMachineModelConfigurer<S, E>>>,
    S: Send + Sync,
    E: Send + Sync,
{
    /// Specify a state machine model factory.
    fn factory(
        &mut self,
        factory: Arc<dyn StateMachineModelFactory<S, E>>,
    ) -> &mut dyn ModelConfigurer<S, E>;
}
