use std::sync::Arc;

use next_web_core::error::BoxError;

use crate::state_machine::config::{
    builders::state_machine_model_configurer::StateMachineModelConfigurer,
    common::{base_configured_builder::BaseConfiguredBuilder, builder::Builder},
    configurers::{
        default_model_configurer::DefaultModelConfigurer, model_configurer::ModelConfigurer,
    },
    model::state_machine_model_factory::StateMachineModelFactory,
};

/// Builder for constructing state machine model configuration.
///
/// # Type Parameters
/// * `S` - The type representing states
/// * `E` - The type representing events
#[derive(Clone)]
pub struct StateMachineModelBuilder<S, E> {
    /// Factory for creating state machine models
    factory: Option<Arc<dyn StateMachineModelFactory<S, E>>>,

    base: BaseConfiguredBuilder<
        ModelData<S, E>,
        Box<dyn StateMachineModelConfigurer<S, E>>,
        StateMachineModelBuilder<S, E>,
    >,
}

impl<S, E> StateMachineModelBuilder<S, E>
where
    S: Send + Sync,
    E: Send + Sync,
{
    /// Creates a new instance of StateMachineModelBuilder.
    pub fn new(allow_configurers_of_same_type: bool) -> Self {
        let base = BaseConfiguredBuilder::with_allow_configurers_of_same_type(
            allow_configurers_of_same_type,
        );

        StateMachineModelBuilder {
            factory: None,
            base,
        }
    }

    // /// Creates a new instance with an object post processor.
    // ///
    // /// # Arguments
    // /// * `object_post_processor` - Processor for post-processing created objects
    // pub fn with_object_post_processor<O>(object_post_processor: O) -> Self
    // where
    //     O: ObjectPostProcessor,
    // {
    //     // Object post processor handling would be implemented here
    //     Self::new()
    // }

    /// Sets the state machine model factory.
    ///
    /// # Arguments
    /// * `factory` - The factory to use for creating state machine models
    pub fn set_state_machine_model_factory<F>(&mut self, factory: F)
    where
        F: StateMachineModelFactory<S, E>,
    {
        self.factory = Some(Arc::new(factory));
    }
}

impl<S, E> Default for StateMachineModelBuilder<S, E> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
/// Represents the configured model data.
pub struct ModelData<S, E> {
    /// Factory for creating state machine models
    factory: Arc<dyn StateMachineModelFactory<S, E>>,
}

impl<S, E> ModelData<S, E> {
    /// Creates new model data with the given factory.
    ///
    /// # Arguments
    /// * `factory` - The state machine model factory
    pub fn new(factory: Arc<dyn StateMachineModelFactory<S, E>>) -> Self {
        ModelData { factory }
    }

    /// Gets a reference to the state machine model factory.
    pub fn factory(&self) -> &dyn StateMachineModelFactory<S, E> {
        &*self.factory
    }
}

impl<S, E> StateMachineModelConfigurer<S, E> for StateMachineModelBuilder<S, E>
where
    S: Send + Sync + 'static,
    S: Clone,
    E: Send + Sync + 'static,
    E: Clone,
{
    fn with_model(&mut self) -> Result<Box<dyn ModelConfigurer<S, E>>, BoxError> {
        let mut configurer = DefaultModelConfigurer::default();
        self.base.apply(configurer);

        Ok(Box::new(configurer))
    }
}

impl<S, E> Builder<ModelData<S, E>> for StateMachineModelBuilder<S, E> {
    fn build(&mut self) -> Result<ModelData<S, E>, BoxError> {
        todo!()
    }
}
