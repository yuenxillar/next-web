use std::sync::Arc;

use crate::config::{
    builders::{
        state_machine_model_builder::StateMachineModelBuilder,
        state_machine_model_configurer::StateMachineModelConfigurer,
    },
    configurer_builder::ConfigurerBuilder, configurers::model_configurer::ModelConfigurer,
    model::state_machine_model_factory::StateMachineModelFactory,
};

/// Default implementation of a `ModelConfigurer`.
///
/// It keeps the model builder it was created from, so [`ConfigurerBuilder::and`]
/// can hand the fluent chain back and the factory reaches the builder.
///
/// # Type Parameters
/// * `S` - The type representing states
/// * `E` - The type representing events
pub struct DefaultModelConfigurer<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// Factory for creating state machine models
    factory: Option<Arc<dyn StateMachineModelFactory<S, E>>>,
    /// The builder this configurer was created from
    builder: Option<StateMachineModelBuilder<S, E>>,
}

impl<S, E> DefaultModelConfigurer<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// Creates a new instance of DefaultModelConfigurer.
    pub fn new() -> Self {
        DefaultModelConfigurer {
            factory: None,
            builder: None,
        }
    }

    /// Keeps the builder which created this configurer.
    ///
    /// # Arguments
    /// * `builder` - the model builder
    pub fn with_builder(mut self, builder: StateMachineModelBuilder<S, E>) -> Self {
        self.builder = Some(builder);
        self
    }
}

impl<S, E> ModelConfigurer<S, E> for DefaultModelConfigurer<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn factory(
        &mut self,
        factory: Arc<dyn StateMachineModelFactory<S, E>>,
    ) -> &mut dyn ModelConfigurer<S, E> {
        if let Some(builder) = self.builder.as_mut() {
            builder.set_state_machine_model_factory_arc(factory.clone());
        }

        self.factory = Some(factory);

        self
    }
}

impl<S, E> ConfigurerBuilder<Box<dyn StateMachineModelConfigurer<S, E>>>
    for DefaultModelConfigurer<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn and(&mut self) -> Box<dyn StateMachineModelConfigurer<S, E>> {
        match self.builder.take() {
            Some(builder) => Box::new(builder),
            None => Box::new(StateMachineModelBuilder::<S, E>::new(true)),
        }
    }
}

impl<S, E> Default for DefaultModelConfigurer<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}
