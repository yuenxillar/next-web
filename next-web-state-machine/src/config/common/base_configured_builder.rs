use next_web_core::anys::any_value::AnyValue;
use next_web_core::clone_box;
use next_web_core::error::BoxError;
use next_web_core::traits::required::Required;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::Arc;
use tracing::error;

use crate::config::common::base_builder::{BaseBuilder, BaseBuilderExt};
use crate::config::common::builder::Builder;
use crate::config::common::configurer::Configurer;
use crate::config::common::configurer_adapter::ConfigurerAdapter;
use crate::config::common::object_post_processor::{ObjectPostProcessor, QuiescentPostProcessor};

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
    shared_objects: Rc<RefCell<HashMap<TypeId, Box<dyn Any>>>>,

    /// Allow configurers of same type
    allow_configurers_of_same_type: bool,

    /// Current build state
    build_state: BuildState,

    /// Object post processor
    object_post_processor: Option<Arc<dyn ObjectPostProcessor>>,

    pub(crate) base: BaseBuilder<O>,
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
            shared_objects: Rc::new(RefCell::new(HashMap::new())),
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
            shared_objects: Rc::new(RefCell::new(HashMap::new())),
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
            shared_objects: Rc::new(RefCell::new(HashMap::new())),
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
    fn add_boxed_configurer(&mut self, type_id: TypeId, configurer: Box<dyn Configurer<O, B>>) {
        if !self.build_state.is_configured() {
            if self.allow_configurers_of_same_type {
                self.main_configurers
                    .entry(type_id)
                    .or_insert_with(Vec::new)
                    .push(configurer);
            } else {
                self.main_configurers.insert(type_id, vec![configurer]);
            }
        } else if self.allow_configurers_of_same_type {
            self.post_configurers
                .entry(type_id)
                .or_insert_with(Vec::new)
                .push(configurer);
        } else {
            self.post_configurers.insert(type_id, vec![configurer]);
        }
    }

    pub fn apply_adapter(
        &mut self,
        configurer: &mut ConfigurerAdapter<O, I, B>,
    ) -> Result<(), BoxError>
    where
        O: 'static,
        I: 'static,
        B: Clone + 'static,
    {
        if let Some(processor) = self.object_post_processor.clone() {
            configurer.add_object_post_processor(processor);
        }

        self.add_boxed_configurer(
            TypeId::of::<ConfigurerAdapter<O, I, B>>(),
            clone_box(configurer),
        );
        Ok(())
    }

    pub fn apply<C>(&mut self, configurer: &mut C) -> Result<(), BoxError>
    where
        C: Configurer<O, B> + 'static,
        O: 'static,
        B: 'static,
    {
        self.add_boxed_configurer(TypeId::of::<C>(), clone_box(configurer));
        Ok(())
    }

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
    pub fn set_shared_object<C: Any + 'static>(&self, object: C) {
        self.shared_objects
            .borrow_mut()
            .insert(TypeId::of::<C>(), Box::new(object));
    }

    /// Executes a closure with a mutable shared object, if present.
    pub fn with_shared_object_mut<C: Any + 'static, R>(
        &self,
        f: impl FnOnce(&mut C) -> R,
    ) -> Option<R> {
        let mut shared_objects = self.shared_objects.borrow_mut();
        let obj = shared_objects.get_mut(&TypeId::of::<C>())?;
        let obj = obj.downcast_mut::<C>()?;
        Some(f(obj))
    }

    /// Executes a closure with an immutable shared object, if present.
    pub fn with_shared_object<C: Any + 'static, R>(&self, f: impl FnOnce(&C) -> R) -> Option<R> {
        let shared_objects = self.shared_objects.borrow();
        let obj = shared_objects.get(&TypeId::of::<C>())?;
        let obj = obj.downcast_ref::<C>()?;
        Some(f(obj))
    }

    pub fn get_own_shared_object<C: Any + 'static>(&self) -> Option<C> {
        self.shared_objects
            .borrow_mut()
            .remove(&TypeId::of::<C>())
            .and_then(|obj| obj.downcast::<C>().ok())
            .map(|obj| *obj)
    }

    /// Gets all shared objects
    pub fn get_shared_objects(&self) -> Rc<RefCell<HashMap<TypeId, Box<dyn Any>>>> {
        self.shared_objects.clone()
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

pub(crate) fn execute_configured_build<T, O, I, B>(builder: &mut T) -> Result<O, BoxError>
where
    T: AsRef<B>,
    T: Required<BaseConfiguredBuilder<O, I, B>>,
    T: BaseConfiguredBuilderExt<O>,
    T: BaseConfiguredBuilderExtOwn<O, I, B>,
    B: Builder<O>,
    B: Clone + 'static,
    O: Clone + 'static,
{
    builder.get_mut_object().build_state = BuildState::InitializingMains;

    builder.get_mut_object().before_init()?;
    builder.init_main_configurers()?;

    builder.get_mut_object().build_state = BuildState::ConfiguringMains;
    builder.get_mut_object().before_configure_mains()?;
    builder.configure_main_configurers()?;

    builder.get_mut_object().build_state = BuildState::ConfiguringPosts;
    builder.get_mut_object().before_configure_posts()?;
    builder.configure_post_configurers()?;

    builder.get_mut_object().build_state = BuildState::Building;
    let result = builder.perform_build()?;

    builder.get_mut_object().build_state = BuildState::Built;
    Ok(result)
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
    fn get_or_apply(&mut self, configurer: &mut ConfigurerAdapter<O, I, B>) -> Result<(), BoxError>
    where
        B: Builder<O>,
        B: Clone,
        B: 'static,
        O: 'static,
        I: 'static,
    {
        if self
            .get_object()
            .get_configurer::<ConfigurerAdapter<O, I, B>>()
            .is_some()
        {
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
        self.add(configurer)?;
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
    fn apply_adapter(&mut self, configurer: &mut ConfigurerAdapter<O, I, B>) -> Result<(), BoxError>
    where
        B: Builder<O>,
        B: Clone,
        B: 'static,
        O: 'static,
        I: 'static,
    {
        self.add(configurer)?;
        self.get_mut_object()
            .object_post_processor
            .as_ref()
            .map(|processor| configurer.add_object_post_processor(processor.clone()));
        configurer.set_builder(self.as_ref().clone());

        Ok(())
    }

    /// Adds a configurer ensuring that it is allowed
    fn add<C>(&mut self, configurer: &mut C) -> Result<(), BoxError>
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
        let mut main_configurers = mem::take(&mut self.get_mut_object().main_configurers);
        let builder = self.as_ref();
        for configurers in main_configurers.values_mut() {
            for configurer in configurers.iter_mut() {
                configurer.init(&builder)?;
            }
        }
        self.get_mut_object().main_configurers = main_configurers;
        Ok(())
    }

    fn configure_main_configurers(&mut self) -> Result<(), BoxError>
    where
        B: Clone,
    {
        let mut main_configurers = mem::take(&mut self.get_mut_object().main_configurers);
        let builder = self.as_ref();
        for configurers in main_configurers.values_mut() {
            for configurer in configurers.iter_mut() {
                configurer.configure(&builder)?;
            }
        }
        self.get_mut_object().main_configurers = main_configurers;
        Ok(())
    }

    fn configure_post_configurers(&mut self) -> Result<(), BoxError>
    where
        B: Clone,
    {
        let mut post_configurers = mem::take(&mut self.get_mut_object().post_configurers);
        let builder = self.as_ref();
        for configurers in post_configurers.values_mut() {
            for configurer in configurers.iter_mut() {
                configurer.configure(&builder)?;
            }
        }
        self.get_mut_object().post_configurers = post_configurers;
        Ok(())
    }
}

impl<T, O, I, B> BaseConfiguredBuilderExtOwn<O, I, B> for T
where
    Self: AsRef<B>,
    Self: Required<BaseConfiguredBuilder<O, I, B>>,

    B: Builder<O>,
{
}

impl<O, I, B> Deref for BaseConfiguredBuilder<O, I, B>
where
    B: Builder<O>,
{
    type Target = BaseBuilder<O>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<O, I, B> DerefMut for BaseConfiguredBuilder<O, I, B>
where
    B: Builder<O>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<O, I, B> Required<BaseBuilder<O>> for BaseConfiguredBuilder<O, I, B>
where
    B: Builder<O>,
{
    fn get_object(&self) -> &BaseBuilder<O> {
        &self.base
    }

    fn get_mut_object(&mut self) -> &mut BaseBuilder<O> {
        &mut self.base
    }
}
