use std::marker::PhantomData;
use std::sync::Arc;

use crate::config::common::builder::Builder;
use crate::config::common::configurer::Configurer;
use crate::config::common::object_post_processor::ObjectPostProcessor;
use next_web_core::anys::any_value::AnyValue;
use next_web_core::error::BoxError;

/// Base adapter for annotation configurers.
/// Allows subclasses to only implement the methods they are interested in.
/// Provides a mechanism for using the AnnotationConfigurer and gaining access to the builder.
///
/// # Type Parameters
/// * `O` - The object being built
/// * `I` - The interface type to return from `and()` method
/// * `B` - The builder that builds O and is configured by this adapter
pub struct ConfigurerAdapter<O, I, B> {
    builder: Option<B>,
    object_post_processor: CompositeObjectPostProcessor,
    _phantom: PhantomData<(O, I)>,
}

impl<O, I, B> ConfigurerAdapter<O, I, B>
where
    B: Builder<O>,
{
    /// Creates a new adapter instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the builder reference. Cannot be None.
    ///
    /// # Panics
    /// Panics if the builder has not been set.
    fn get_builder(&mut self) -> Option<&mut B> {
        self.builder.as_mut()
    }

    /// Adds an ObjectPostProcessor to be used by this adapter.
    ///
    /// # Arguments
    /// * `processor` - The ObjectPostProcessor to add
    pub fn add_object_post_processor(&mut self, processor: Arc<dyn ObjectPostProcessor>) {
        self.object_post_processor
            .add_object_post_processor(processor);
    }

    /// Sets the builder to be used.
    ///
    /// # Arguments
    /// * `builder` - The builder to set
    pub fn set_builder(&mut self, builder: B) {
        self.builder = Some(builder);
    }

    /// Takes the builder, consuming self.
    ///
    /// # Returns
    /// The builder if it exists, None otherwise.
    pub fn take_builder(&mut self) -> Option<B> {
        self.builder.take()
    }
}

impl<O, I, B> Default for ConfigurerAdapter<O, I, B>
where
    B: Builder<O>,
{
    fn default() -> Self {
        Self {
            builder: None,
            object_post_processor: CompositeObjectPostProcessor::default(),
            _phantom: PhantomData,
        }
    }
}

impl<O, I, B> Clone for ConfigurerAdapter<O, I, B>
where
    B: Builder<O>,
    B: Clone,
{
    fn clone(&self) -> Self {
        Self {
            builder: self.builder.clone(),
            object_post_processor: self.object_post_processor.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<O, I, B> Configurer<O, B> for ConfigurerAdapter<O, I, B>
where
    B: Builder<O>,
    B: Clone,
{
    fn init(&mut self, _builder: &B) -> Result<(), BoxError> {
        Ok(())
    }

    fn configure(&mut self, _builder: &B) -> Result<(), BoxError> {
        Ok(())
    }

    fn is_assignable(&self, _builder: &B) -> bool {
        true
    }
}

/// A composite ObjectPostProcessor that delegates work to multiple ObjectPostProcessor implementations.
#[derive(Clone)]
pub struct CompositeObjectPostProcessor {
    post_processors: Vec<Arc<dyn ObjectPostProcessor>>,
}

impl CompositeObjectPostProcessor {
    /// Adds an ObjectPostProcessor to use.
    ///
    /// # Arguments
    /// * `processor` - The ObjectPostProcessor to add
    fn add_object_post_processor(&mut self, processor: Arc<dyn ObjectPostProcessor>) {
        self.post_processors.push(processor);
    }
}

impl ObjectPostProcessor for CompositeObjectPostProcessor {
    /// Post-processes an object through all registered processors.
    ///
    /// # Arguments
    /// * `object` - The object to post-process
    ///
    /// # Returns
    /// The post-processed object
    fn post_process(&self, object: &mut AnyValue) {
        for processor in self.post_processors.iter() {
            processor.post_process(object);
        }
    }
}

impl Default for CompositeObjectPostProcessor {
    fn default() -> Self {
        Self {
            post_processors: Default::default(),
        }
    }
}

pub trait ConfigurerAdapterExt<O, B>
where
    B: Builder<O>,
{
    fn configure(&mut self, builder: &mut B) -> Result<(), BoxError>;
}
