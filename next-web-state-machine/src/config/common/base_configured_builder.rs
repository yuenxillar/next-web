use next_web_core::Required;
use next_web_core::any::AnyValue;
use next_web_core::clone_box;
use next_web_core::error::BoxError;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;
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
    main_configurers: Rc<RefCell<BTreeMap<TypeId, Vec<Box<dyn Configurer<O, B>>>>>>,

    /// Configurers which are added during the configuration phase
    post_configurers: Rc<RefCell<BTreeMap<TypeId, Vec<Box<dyn Configurer<O, B>>>>>>,

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
            main_configurers: Rc::new(RefCell::new(BTreeMap::new())),
            post_configurers: Rc::new(RefCell::new(BTreeMap::new())),
            shared_objects: Rc::new(RefCell::new(HashMap::new())),
            allow_configurers_of_same_type: false,
            build_state: BuildState::Unbuilt,
            object_post_processor: Some(Arc::new(object_post_processor)),
            base: BaseBuilder::default(),
            _i: PhantomData,
        }
    }

    /// Creates a new builder which allows several configurers of the same type.
    ///
    /// # Arguments
    /// * `allow_configurers_of_same_type` - whether several configurers of one type are kept
    pub fn with_allow_configurers_of_same_type(allow_configurers_of_same_type: bool) -> Self {
        Self {
            main_configurers: Rc::new(RefCell::new(BTreeMap::new())),
            post_configurers: Rc::new(RefCell::new(BTreeMap::new())),
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
            main_configurers: Rc::new(RefCell::new(BTreeMap::new())),
            post_configurers: Rc::new(RefCell::new(BTreeMap::new())),
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
        let mut bucket = if self.build_state.is_configured() {
            self.post_configurers.borrow_mut()
        } else {
            self.main_configurers.borrow_mut()
        };

        if self.allow_configurers_of_same_type {
            bucket.entry(type_id).or_default().push(configurer);
        } else {
            bucket.insert(type_id, vec![configurer]);
        }
    }

    /// Registers a configurer adapter, so it is applied during the build.
    ///
    /// # Arguments
    /// * `configurer` - the configurer adapter to register
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

    /// Registers a configurer, so it is applied during the build.
    ///
    /// # Arguments
    /// * `configurer` - the configurer to register
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
    ///
    /// Returns `None` when the type is not registered, and also when several
    /// configurers of the type are registered, because the caller asked for the
    /// one of its type. Use [`Self::get_configurers`] for the latter case.
    pub fn get_configurer<C>(&self) -> Option<Box<dyn Configurer<O, B>>>
    where
        C: Configurer<O, B> + 'static,
    {
        let type_id = TypeId::of::<C>();
        let main_configurers = self.main_configurers.borrow();

        let configs = main_configurers.get(&type_id)?;

        if configs.len() != 1 {
            error!(
                count = configs.len(),
                "several configurers are registered for the requested type"
            );
            return None;
        }

        configs
            .first()
            .map(|configurer| clone_box(configurer.as_ref()))
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

    /// Takes the shared object of the given type out of the shared map.
    ///
    /// # Type Parameters
    /// * `C` - the type of the shared object
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
    pub fn get_configurers<C>(&self) -> Vec<Box<dyn Configurer<O, B>>>
    where
        C: Configurer<O, B> + Clone + 'static,
    {
        let type_id = TypeId::of::<C>();
        let main_configurers = self.main_configurers.borrow();

        match main_configurers.get(&type_id) {
            Some(configs) => configs
                .iter()
                .map(|configurer| clone_box(configurer.as_ref()))
                .collect::<Vec<_>>(),
            None => Default::default(),
        }
    }

    /// Removes all configurer instances by type
    pub fn remove_configurers<C>(&mut self) -> Vec<Box<dyn Configurer<O, B>>>
    where
        C: Configurer<O, B> + 'static,
    {
        let type_id = TypeId::of::<C>();

        self.main_configurers
            .borrow_mut()
            .remove(&type_id)
            .unwrap_or_default()
    }

    /// Removes and returns a configurer by type
    ///
    /// When several configurers of the type are registered the first one is
    /// removed and the rest are kept, which the error log reports; the builder
    /// never panics for it.
    pub fn remove_configurer<C>(&mut self) -> Option<Box<dyn Configurer<O, B>>>
    where
        C: Configurer<O, B> + 'static,
    {
        let type_id = TypeId::of::<C>();
        let mut configs = self.main_configurers.borrow_mut();
        let mut registered = configs.remove(&type_id)?;

        if registered.len() != 1 {
            error!(
                count = registered.len(),
                "several configurers are registered for the requested type"
            );
            let first = registered.remove(0);
            configs.insert(type_id, registered);

            return Some(first);
        }

        registered.pop()
    }

    /// Sets the post processor applied to the objects of the build.
    ///
    /// # Arguments
    /// * `object_post_processor` - the post processor
    pub fn set_object_post_processor(
        &mut self,
        object_post_processor: Arc<dyn ObjectPostProcessor>,
    ) {
        self.object_post_processor = Some(object_post_processor);
    }

    /// Applies the post processor to a value, when one is configured.
    ///
    /// # Arguments
    /// * `value` - the value to post process
    pub fn post_process(&self, value: &mut AnyValue) {
        if let Some(object_post_processor) = self.object_post_processor.as_ref() {
            object_post_processor.post_process(value);
        }
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
    /// Determines whether the build reached the configuring phase or later.
    ///
    /// Configurers registered from here on belong to the post bucket, so they do
    /// not join the bucket which is being iterated.
    pub fn is_configured(&self) -> bool {
        self >= &Self::ConfiguringMains
    }

    /// Checks whether the builder is initializing its main configurers.
    pub fn is_initializing(&self) -> bool {
        self == &Self::InitializingMains
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

/// Wraps a builder so the type parameters the configured builder needs are
/// explicit at the call site.
pub struct WithTypes<T, I, B> {
    inner: T,
    _marker: std::marker::PhantomData<(I, B)>,
}

impl<T, I, B> WithTypes<T, I, B> {
    /// Wraps the given builder.
    ///
    /// # Arguments
    /// * `inner` - the builder to wrap
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

    // The configurers are dropped once the object is built: a configurer may
    // hold a clone of the builder it belongs to, so keeping them registered
    // would leave a reference cycle behind, and the build reads its data from
    // the shared objects instead of the registry.
    builder.get_mut_object().release_configurers();

    builder.get_mut_object().build_state = BuildState::Built;
    Ok(result)
}

impl<O, I, B> BaseConfiguredBuilder<O, I, B>
where
    B: Builder<O>,
{
    /// Drops the registered configurers of both buckets.
    fn release_configurers(&mut self) {
        self.main_configurers.borrow_mut().clear();
        self.post_configurers.borrow_mut().clear();
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

/// Callback which builds the object after every configurer has run.
pub trait BaseConfiguredBuilderExt<O> {
    /// Builds the object.
    fn perform_build(&mut self) -> Result<O, BoxError>;
}

/// Extends a builder with the registration helpers of the configuration layer.
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

    /// Similar to `build()` and `get_object()`, but checks the build state to
    /// determine whether `build()` has to be called first.
    ///
    /// # Returns
    /// The built object, or `None` when the build failed
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
        if let Some(processor) = self.get_mut_object()
            .object_post_processor
            .as_ref() { configurer.add_object_post_processor(processor.clone()) }
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
            // The registry is keyed by the configurer type, so a lookup by type
            // finds it again and two different configurers do not share a slot.
            let type_id = TypeId::of::<C>();

            if self.get_object().build_state.is_initializing() {
                configurer.init(self.as_ref())?;
            }

            if self.get_object().allow_configurers_of_same_type {
                self.get_mut_object()
                    .main_configurers
                    .borrow_mut()
                    .entry(type_id)
                    .or_default()
                    .push(clone_box(configurer));
            } else {
                self.get_mut_object()
                    .main_configurers
                    .borrow_mut()
                    .insert(type_id, vec![clone_box(configurer)]);
            }
        } else {
            let type_id = TypeId::of::<C>();

            configurer.init(self.as_ref())?;
            if self.get_object().allow_configurers_of_same_type {
                self.get_mut_object()
                    .post_configurers
                    .borrow_mut()
                    .entry(type_id)
                    .or_default()
                    .push(clone_box(configurer));
            } else {
                self.get_mut_object()
                    .post_configurers
                    .borrow_mut()
                    .insert(type_id, vec![clone_box(configurer)]);
            }
        }

        Ok(())
    }

    /// Initializes the main configurers.
    fn init_main_configurers(&mut self) -> Result<(), BoxError> {
        // Iterate a snapshot so a configurer which registers another one while it
        // is initialized does not lose that registration.
        let mut main_configurers = self.snapshot_main_configurers();
        let builder = self.as_ref();
        for configurer in main_configurers.iter_mut() {
            configurer.init(builder)?;
        }
        Ok(())
    }

    /// Configures the main configurers.
    fn configure_main_configurers(&mut self) -> Result<(), BoxError>
    where
        B: Clone,
    {
        let mut main_configurers = self.snapshot_main_configurers();
        let builder = self.as_ref();
        for configurer in main_configurers.iter_mut() {
            configurer.configure(builder)?;
        }
        Ok(())
    }

    /// Configures the post configurers, which are registered while the main ones
    /// are configured.
    fn configure_post_configurers(&mut self) -> Result<(), BoxError>
    where
        B: Clone,
    {
        let mut post_configurers: Vec<Box<dyn Configurer<O, B>>> = self
            .get_object()
            .post_configurers
            .borrow()
            .values()
            .flatten()
            .map(|configurer| clone_box(configurer.as_ref()))
            .collect();
        let builder = self.as_ref();
        for configurer in post_configurers.iter_mut() {
            configurer.configure(builder)?;
        }
        Ok(())
    }

    /// Clones the main configurers so they can be iterated while the registry
    /// stays available for registrations.
    fn snapshot_main_configurers(&self) -> Vec<Box<dyn Configurer<O, B>>> {
        self.get_object()
            .main_configurers
            .borrow()
            .values()
            .flatten()
            .map(|configurer| clone_box(configurer.as_ref()))
            .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::common::builder::Builder;

    #[derive(Clone, Default)]
    struct TestBuilder;

    impl Builder<u32> for TestBuilder {
        fn build(&mut self) -> Result<u32, BoxError> {
            Ok(1)
        }
    }

    #[derive(Clone)]
    struct MarkerConfigurer;

    impl Configurer<u32, TestBuilder> for MarkerConfigurer {
        fn init(&mut self, _builder: &TestBuilder) -> Result<(), BoxError> {
            Ok(())
        }

        fn configure(&mut self, _builder: &TestBuilder) -> Result<(), BoxError> {
            Ok(())
        }

        fn is_assignable(&self, _builder: &TestBuilder) -> bool {
            true
        }
    }

    type TestConfiguredBuilder =
        BaseConfiguredBuilder<u32, Box<dyn Configurer<u32, TestBuilder>>, TestBuilder>;

    #[test]
    fn build_state_matches_the_configuration_phases() {
        assert!(!BuildState::Unbuilt.is_configured());
        assert!(!BuildState::InitializingMains.is_configured());
        assert!(BuildState::ConfiguringMains.is_configured());
        assert!(BuildState::ConfiguringPosts.is_configured());
        assert!(BuildState::Building.is_configured());
        assert!(BuildState::Built.is_configured());

        assert!(BuildState::InitializingMains.is_initializing());
        assert!(!BuildState::ConfiguringMains.is_initializing());
        assert!(!BuildState::Unbuilt.is_initializing());
    }

    #[test]
    fn configurers_registered_while_configuring_go_to_the_post_bucket() {
        let mut builder = TestConfiguredBuilder::with_allow_configurers_of_same_type(true);

        builder.build_state = BuildState::ConfiguringMains;
        builder
            .apply(&mut MarkerConfigurer)
            .expect("the configurer is registered");

        assert!(
            builder.get_configurer::<MarkerConfigurer>().is_none(),
            "a configurer registered during the configure phase must not join the main bucket"
        );
        assert_eq!(
            builder.post_configurers.borrow().len(),
            1,
            "it must be queued in the post bucket instead"
        );
    }

    #[test]
    fn several_configurers_of_one_type_do_not_panic_the_lookup() {
        let mut builder = TestConfiguredBuilder::with_allow_configurers_of_same_type(true);

        builder
            .apply(&mut MarkerConfigurer)
            .expect("the first configurer is registered");
        builder
            .apply(&mut MarkerConfigurer)
            .expect("the second configurer is registered");

        assert!(
            builder.get_configurer::<MarkerConfigurer>().is_none(),
            "a lookup for the single configurer of a type returns None when there are several"
        );
        assert_eq!(builder.get_configurers::<MarkerConfigurer>().len(), 2);
        assert!(builder.remove_configurer::<MarkerConfigurer>().is_some());
        assert_eq!(
            builder.get_configurers::<MarkerConfigurer>().len(),
            1,
            "removing one configurer keeps the rest"
        );
    }

    #[test]
    fn a_built_builder_releases_its_configurers() {
        use crate::config::builders::state_machine_state_builder::StateMachineStateBuilder;
        use crate::config::model::states_data::StatesData;

        #[derive(Clone)]
        struct PlainStatesConfigurer;

        impl Configurer<StatesData<i32, i32>, StateMachineStateBuilder<i32, i32>>
            for PlainStatesConfigurer
        {
            fn init(
                &mut self,
                _builder: &StateMachineStateBuilder<i32, i32>,
            ) -> Result<(), BoxError> {
                Ok(())
            }

            fn configure(
                &mut self,
                _builder: &StateMachineStateBuilder<i32, i32>,
            ) -> Result<(), BoxError> {
                Ok(())
            }

            fn is_assignable(&self, _builder: &StateMachineStateBuilder<i32, i32>) -> bool {
                true
            }
        }

        let mut builder = StateMachineStateBuilder::<i32, i32>::new(true);

        builder
            .apply(&mut PlainStatesConfigurer)
            .expect("the configurer is registered");

        assert_eq!(
            builder.get_configurers::<PlainStatesConfigurer>().len(),
            1,
            "the builder keeps the registered configurer"
        );

        builder
            .build()
            .expect("the object is built without an error");

        assert!(
            builder.get_configurers::<PlainStatesConfigurer>().is_empty(),
            "the configurers are released after the build, which breaks the cycle a configurer creates when it holds its builder"
        );
    }
}
