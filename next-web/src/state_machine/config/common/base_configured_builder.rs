use next_web_core::anys::any_value::AnyValue;
use next_web_core::error::BoxError;
use next_web_core::traits::any_clone::AnyClone;
use next_web_core::traits::required::Required;
use next_web_core::{clone_box, DynClone};
use std::any::{Any, TypeId};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;
use tracing::error;

use crate::state_machine::config::common::base_builder::{BaseBuilder, BaseBuilderExt};
use crate::state_machine::config::common::builder::Builder;
use crate::state_machine::config::common::configurer::Configurer;
use crate::state_machine::config::common::configurer_adapter::ConfigurerAdapter;
use crate::state_machine::config::common::object_post_processor::{
    ObjectPostProcessor, QuiescentPostProcessor,
};

/// A base AnnotationBuilder that allows AnnotationConfigurers to be applied to it
#[derive(Clone)]
pub struct BaseConfiguredBuilder<O, I, B>
where
    B: Builder<O>,
{
    /// Configurers which are added before the configure step
    main_configurers: BTreeMap<TypeId, Vec<Box<dyn Configurer<O, B>>>>,

    /// Configurers which are added during the configuration phase
    post_configurers: BTreeMap<TypeId, Vec<Box<dyn Configurer<O, B>>>>,

    /// Shared objects
    shared_objects: HashMap<TypeId, AnyValue>,

    /// Allow configurers of same type
    allow_configurers_of_same_type: bool,

    /// Current build state
    build_state: BuildState,

    /// Object post processor
    object_post_processor: Option<Arc<dyn ObjectPostProcessor>>,

    base: BaseBuilder<O>,
    _i: PhantomData<I>,
}

impl<O, I, B> BaseConfiguredBuilder<O, I, B>
where
    B: Builder<O>,
    I: 'static,
{
    /// Creates a new builder with specified post processor
    pub fn with_post_processor<T: ObjectPostProcessor>(object_post_processor: T) -> Self {
        Self {
            main_configurers: BTreeMap::new(),
            post_configurers: BTreeMap::new(),
            shared_objects: HashMap::new(),
            allow_configurers_of_same_type: false,
            build_state: BuildState::Unbuilt,
            object_post_processor: Some(Arc::new(object_post_processor)),
            base: BaseBuilder::default(),
            _i: PhantomData,
        }
    }

    pub fn with_allow_configurers_of_same_type(allow_configurers_of_same_type: bool) -> Self {
        Self {
            main_configurers: BTreeMap::new(),
            post_configurers: BTreeMap::new(),
            shared_objects: HashMap::new(),
            allow_configurers_of_same_type,
            build_state: BuildState::Unbuilt,
            object_post_processor: None,
            base: BaseBuilder::default(),
            _i: PhantomData,
        }
    }

    /// Creates a new builder with specified post processor and same-type configurer allowance
    pub fn with_post_processor_and_allow_same_type<T: ObjectPostProcessor>(
        object_post_processor: T,
        allow_configurers_of_same_type: bool,
    ) -> Self {
        Self {
            main_configurers: BTreeMap::new(),
            post_configurers: BTreeMap::new(),
            shared_objects: HashMap::new(),
            allow_configurers_of_same_type,
            build_state: BuildState::Unbuilt,
            object_post_processor: Some(Arc::new(object_post_processor)),
            base: BaseBuilder::default(),
            _i: PhantomData,
        }
    }
}

impl<O, I, B> BaseConfiguredBuilder<O, I, B>
where
    B: Builder<O>,
{
    /// Gets a configurer by type
    pub fn get_configurer<C>(&self) -> Option<&Box<dyn Configurer<O, B>>>
    where
        C: Configurer<O, B> + 'static,
    {
        let type_id = TypeId::of::<C>();

        if let Some(configs) = self.main_configurers.get(&type_id) {
            if configs.len() != 1 {
                panic!(
                    "Only one configurer expected for type, but got {}",
                    configs.len()
                )
            } else {
                return configs.get(0);
            }
        }

        None
    }

    /// Invoked prior to invoking each init() method
    pub fn before_init(&self) -> Result<(), BoxError> {
        Ok(())
    }

    /// Invoked prior to invoking each main configure() method
    pub fn before_configure_mains(&self) -> Result<(), BoxError> {
        Ok(())
    }

    /// Invoked prior to invoking each post configure() method
    pub fn before_configure_posts(&self) -> Result<(), BoxError> {
        Ok(())
    }

    /// Sets an object that is shared by multiple AnnotationConfigurers
    pub fn set_shared_object<C: AnyClone>(&mut self, object: C) {
        self.shared_objects
            .insert(TypeId::of::<C>(), AnyValue::Object(Box::new(object)));
    }

    /// Gets a shared object
    pub fn get_shared_object<'a, C: Any>(&'a self) -> Option<&'a C> {
        self.shared_objects
            .get(&TypeId::of::<C>())
            .map(|obj| obj.as_ref_object::<C>())
            .unwrap_or_default()
    }

    /// Gets all shared objects
    pub fn get_shared_objects(&self) -> &HashMap<TypeId, AnyValue> {
        &self.shared_objects
    }

    /// Gets all configurer instances by type
    pub fn get_configurers<C>(&self) -> Vec<&Box<dyn Configurer<O, B>>>
    where
        C: Configurer<O, B> + Clone + 'static,
    {
        let type_id = TypeId::of::<C>();

        match self.main_configurers.get(&type_id) {
            Some(configs) => configs.iter().collect::<Vec<_>>(),
            None => Default::default(),
        }
    }

    /// Removes all configurer instances by type
    pub fn remove_configurers<C>(&mut self) -> Vec<Box<dyn Configurer<O, B>>>
    where
        C: Configurer<O, B> + 'static,
    {
        let type_id = TypeId::of::<C>();

        match self.main_configurers.remove(&type_id) {
            Some(configs) => configs,
            None => Default::default(),
        }
    }

    /// Removes and returns a configurer by type
    pub fn remove_configurer<C>(&mut self) -> Option<Box<dyn Configurer<O, B>>>
    where
        C: Configurer<O, B> + 'static,
    {
        let type_id = TypeId::of::<C>();

        if let Some(mut configs) = self.main_configurers.remove(&type_id) {
            if configs.len() != 1 {
                panic!(
                    "Only one configurer expected for type, but got {}",
                    configs.len()
                );
            } else {
                return Some(configs.remove(0));
            }
        }

        None
    }

    pub fn set_object_post_processor(
        &mut self,
        object_post_processor: Arc<dyn ObjectPostProcessor>,
    ) {
        self.object_post_processor = Some(object_post_processor);
    }

    pub fn post_process(&self, value: &mut AnyValue) {
        self.object_post_processor
            .as_ref()
            .map(|object_post_processor| {
                object_post_processor.post_process(value);
            });
    }

    fn get_main_configurers(&mut self) -> Vec<&mut Box<dyn Configurer<O, B>>> {
        self.main_configurers
            .values_mut()
            .into_iter()
            .fold(Vec::new(), |mut acc, var| {
                var.iter_mut().map(|var2| acc.push(var2));
                acc
            })
    }

    fn get_post_configurers(&mut self) -> Vec<&mut Box<dyn Configurer<O, B>>> {
        self.post_configurers
            .values_mut()
            .into_iter()
            .fold(Vec::new(), |mut acc, var| {
                var.iter_mut().map(|var2| acc.push(var2));
                acc
            })
    }

    fn is_unbuilt(&self) -> bool {
        self.build_state == BuildState::Unbuilt
    }
}

/// Build state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BuildState {
    /// This is the state before the build() is invoked
    Unbuilt = 0,

    /// The state from when build() is first invoked until all the init() methods have been invoked
    InitializingMains = 1,

    /// The state from after all main init() have been invoked until after all the configure() methods
    ConfiguringMains = 2,

    /// The state from after all post init() have been invoked until after all the configure() methods
    ConfiguringPosts = 3,

    /// From the point after all the configure() have completed to just after perform_build()
    Building = 4,

    /// After the object has been completely built
    Built = 5,
}

impl BuildState {
    pub fn is_configured(&self) -> bool {
        self == &Self::InitializingMains
    }

    pub fn is_initializing(&self) -> bool {
        self == &Self::ConfiguringMains
    }
}

impl<O, I, B> Default for BaseConfiguredBuilder<O, I, B>
where
    B: Builder<O> + 'static,
    O: Clone + 'static,
    I: 'static,
{
    fn default() -> Self {
        Self::with_post_processor(QuiescentPostProcessor)
    }
}

pub struct WithTypes<T, I, B> {
    inner: T,
    _marker: std::marker::PhantomData<(I, B)>,
}

impl<T, I, B> WithTypes<T, I, B> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T, O, I, B> BaseBuilderExt<O> for WithTypes<T, I, B>
where
    T: Required<BaseConfiguredBuilder<O, I, B>>,
    T: BaseConfiguredBuilderExt<O>,
    T: BaseConfiguredBuilderExtOwn<O, I, B>,
    B: Builder<O>,
    B: Clone + 'static,
    O: Clone + 'static,
{
    fn do_build(&mut self) -> Result<O, BoxError> {
        self.get_mut_object().build_state = BuildState::InitializingMains;

        self.get_mut_object().before_init()?;
        self.inner.init_main_configurers()?;

        self.get_mut_object().build_state = BuildState::ConfiguringMains;
        self.get_mut_object().before_configure_mains()?;
        self.inner.configure_main_configurers()?;

        self.get_mut_object().build_state = BuildState::ConfiguringPosts;
        self.get_mut_object().before_configure_posts()?;
        self.inner.configure_post_configurers()?;

        self.get_mut_object().build_state = BuildState::Building;
        let result = self.inner.perform_build()?;

        self.get_mut_object().build_state = BuildState::Built;
        Ok(result)
    }
}

impl<T, O, I, B> Required<BaseConfiguredBuilder<O, I, B>> for WithTypes<T, I, B>
where
    T: Required<BaseConfiguredBuilder<O, I, B>>,
    B: Builder<O>,
{
    fn get_object(&self) -> &BaseConfiguredBuilder<O, I, B> {
        self.inner.get_object()
    }
    fn get_mut_object(&mut self) -> &mut BaseConfiguredBuilder<O, I, B> {
        self.inner.get_mut_object()
    }
}

pub trait BaseConfiguredBuilderExt<O> {
    fn perform_build(&mut self) -> Result<O, BoxError>;
}

pub trait BaseConfiguredBuilderExtOwn<O, I, B>
where
    Self: AsRef<B>,
    Self: Required<BaseConfiguredBuilder<O, I, B>>,

    B: Builder<O>,
{
    /// Similar to apply_adapter but checks if configurer already exists
    fn get_or_apply<C>(&mut self, configurer: &mut C) -> Result<(), BoxError>
    where
        C: Required<ConfigurerAdapter<O, I, B>>,
        C: DynClone,
        C: Configurer<O, B> + 'static,
        B: Builder<O>,
        B: Clone,
        B: 'static,
    {
        let existing = self.get_object().get_configurer::<C>();
        if let Some(existing) = existing {
            return Ok(());
        }

        self.apply_adapter(configurer)
    }

    /// Applies an AnnotationConfigurerAdapter to this builder
    fn apply<C>(&mut self, configurer: &mut C) -> Result<(), BoxError>
    where
        C: Configurer<O, B>,
        B: 'static,
        C: 'static,
        O: 'static,
    {
        self.add(configurer);
        Ok(())
    }

    /// Similar to build() and get_object() but checks the state to determine if build() needs to Required<BaseBuilder<O>>be called first
    fn get_or_build(&mut self) -> Option<O>
    where
        O: Clone,
        Self: Builder<O>,
    {
        if self.get_object().is_unbuilt() {
            match self.build() {
                Ok(obj) => Some(obj),
                Err(e) => {
                    error!("Failed to perform build. Returning None: {}", e);
                    None
                }
            }
        } else {
            self.get_object().base.get_object()
        }
    }

    /// Applies an AnnotationConfigurer to this builder
    fn apply_adapter<C>(&mut self, configurer: &mut C) -> Result<(), BoxError>
    where
        C: Required<ConfigurerAdapter<O, I, B>>,
        C: DynClone,
        B: Builder<O>,
        B: Clone,
        C: 'static,
        B: 'static,
        I: 'static,
    {
        self.add(configurer.get_mut_object());
        self.get_mut_object()
            .object_post_processor
            .as_ref()
            .map(|processor| {
                configurer
                    .get_mut_object()
                    .add_object_post_processor(processor.clone())
            });
        configurer
            .get_mut_object()
            .set_builder(self.as_ref().clone());

        Ok(())
    }

    /// Adds a configurer ensuring that it is allowed
    fn add<C>(&mut self, mut configurer: &mut C) -> Result<(), BoxError>
    where
        C: Configurer<O, B>,
        C: 'static,
        B: Builder<O>,
        B: 'static,
        O: 'static,
    {
        if !self.get_object().build_state.is_configured() {
            let type_id = TypeId::of::<Box<dyn Configurer<O, B>>>();

            if self.get_object().allow_configurers_of_same_type {
                if self.get_object().build_state.is_initializing() {
                    configurer.init(self.as_ref())?;
                }
                let configs = self
                    .get_mut_object()
                    .main_configurers
                    .entry(type_id)
                    .or_insert_with(Vec::new);
                configs.push(clone_box(configurer));
            } else {
                if self.get_object().build_state.is_initializing() {
                    configurer.init(self.as_ref())?;
                }
                self.get_mut_object()
                    .main_configurers
                    .insert(type_id, vec![clone_box(configurer)]);
            }
        } else {
            let type_id = TypeId::of::<Box<dyn Configurer<O, B>>>();

            configurer.init(self.as_ref())?;
            if self.get_object().allow_configurers_of_same_type {
                let configs = self
                    .get_mut_object()
                    .post_configurers
                    .entry(type_id)
                    .or_insert_with(Vec::new);
                configs.push(clone_box(configurer));
            } else {
                self.get_mut_object()
                    .post_configurers
                    .insert(type_id, vec![clone_box(configurer)]);
            }
        }

        Ok(())
    }

    fn init_main_configurers(&mut self) -> Result<(), BoxError> {
        for configurer in self.get_mut_object().get_main_configurers() {
            configurer.init(self.as_ref())?;
        }
        Ok(())
    }

    fn configure_main_configurers(&mut self) -> Result<(), BoxError> {
        for configurer in self.get_mut_object().get_main_configurers() {
            configurer.configure(self.as_ref())?;
        }
        Ok(())
    }

    fn configure_post_configurers(&mut self) -> Result<(), BoxError> {
        for configurer in self.get_mut_object().get_post_configurers() {
            configurer.configure(self.as_ref())?;
        }
        Ok(())
    }
}
