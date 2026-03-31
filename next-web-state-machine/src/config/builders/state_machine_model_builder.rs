use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use next_web_core::{error::BoxError, traits::required::Required};

use crate::config::{
    builders::state_machine_model_configurer::StateMachineModelConfigurer,
    common::{
        base_builder::{BaseBuilder, BaseBuilderExt},
        base_configured_builder::{
            BaseConfiguredBuilder, BaseConfiguredBuilderExt, execute_configured_build,
        },
        builder::Builder,
    },
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
pub struct StateMachineModelBuilder<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
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
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
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
        F: StateMachineModelFactory<S, E> + 'static,
    {
        self.factory = Some(Arc::new(factory));
    }
}

impl<S, E> Default for StateMachineModelBuilder<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self {
            factory: None,
            base: BaseConfiguredBuilder::with_allow_configurers_of_same_type(false),
        }
    }
}

#[derive(Clone)]
/// Represents the configured model data.
pub struct ModelData<S, E> {
    /// Factory for creating state machine models
    factory: Option<Arc<dyn StateMachineModelFactory<S, E>>>,
}

impl<S, E> ModelData<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// Creates new model data with the given factory.
    ///
    /// # Arguments
    /// * `factory` - The state machine model factory
    pub fn new(factory: Option<Arc<dyn StateMachineModelFactory<S, E>>>) -> Self {
        ModelData { factory }
    }

    /// Gets a reference to the state machine model factory.
    pub fn factory(&self) -> Option<&Arc<dyn StateMachineModelFactory<S, E>>> {
        self.factory.as_ref()
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
        let configurer = DefaultModelConfigurer::default();

        Ok(Box::new(configurer))
    }
}

impl<S, E> BaseConfiguredBuilderExt<ModelData<S, E>> for StateMachineModelBuilder<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn perform_build(&mut self) -> Result<ModelData<S, E>, BoxError> {
        let factory = self.factory.clone();

        Ok(ModelData::new(factory))
    }
}

impl<S, E> BaseBuilderExt<ModelData<S, E>> for StateMachineModelBuilder<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn do_build(&mut self) -> Result<ModelData<S, E>, BoxError> {
        execute_configured_build(self)
    }
}

impl<S, E> AsRef<Self> for StateMachineModelBuilder<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<S, E> Required<BaseBuilder<ModelData<S, E>>> for StateMachineModelBuilder<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn get_object(&self) -> &BaseBuilder<ModelData<S, E>> {
        &self.base.base
    }

    fn get_mut_object(&mut self) -> &mut BaseBuilder<ModelData<S, E>> {
        &mut self.base.base
    }
}

impl<S, E> Deref for StateMachineModelBuilder<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    type Target =
        BaseConfiguredBuilder<ModelData<S, E>, Box<dyn StateMachineModelConfigurer<S, E>>, Self>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<S, E> DerefMut for StateMachineModelBuilder<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<S, E>
    Required<
        BaseConfiguredBuilder<ModelData<S, E>, Box<dyn StateMachineModelConfigurer<S, E>>, Self>,
    > for StateMachineModelBuilder<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn get_object(
        &self,
    ) -> &BaseConfiguredBuilder<ModelData<S, E>, Box<dyn StateMachineModelConfigurer<S, E>>, Self>
    {
        &self.base
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut BaseConfiguredBuilder<ModelData<S, E>, Box<dyn StateMachineModelConfigurer<S, E>>, Self>
    {
        &mut self.base
    }
}
