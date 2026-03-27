use std::sync::Arc;

use crate::config::{
    builders::state_machine_model_configurer::StateMachineModelConfigurer,
    configurer_builder::ConfigurerBuilder, configurers::model_configurer::ModelConfigurer,
    model::state_machine_model_factory::StateMachineModelFactory,
};

/// Default implementation of a `ModelConfigurer`.
///
/// # Type Parameters
/// * `S` - The type representing states
/// * `E` - The type representing events
pub struct DefaultModelConfigurer<S, E> {
    /// Factory for creating state machine models
    factory: Option<Arc<dyn StateMachineModelFactory<S, E>>>,
}

impl<S, E> DefaultModelConfigurer<S, E> {
    /// Creates a new instance of DefaultModelConfigurer.
    pub fn new() -> Self {
        DefaultModelConfigurer { factory: None }
    }
}

impl<S, E> ModelConfigurer<S, E> for DefaultModelConfigurer<S, E>
where
    S: Send + Sync,
    E: Send + Sync,
{
    fn factory(
        &mut self,
        factory: Arc<dyn StateMachineModelFactory<S, E>>,
    ) -> &mut dyn ModelConfigurer<S, E> {
        self.factory = Some(factory);

        self
    }
}

impl<S, E> ConfigurerBuilder<Box<dyn StateMachineModelConfigurer<S, E>>>
    for DefaultModelConfigurer<S, E>
{
    fn and(&mut self) -> Box<dyn StateMachineModelConfigurer<S, E>> {
        todo!()
    }
}

impl<S, E> Default for DefaultModelConfigurer<S, E> {
    fn default() -> Self {
        Self { factory: None }
    }
}
