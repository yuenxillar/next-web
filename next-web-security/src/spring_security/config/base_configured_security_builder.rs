use std::any::{type_name, Any, TypeId};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::{collections::BTreeMap, marker::PhantomData};

use crate::config::security_configurer::SecurityConfigurer;
use crate::config::{
    base_security_builder::BaseSecurityBuilder, security_builder::SecurityBuilder,
};
use next_web_core::traits::any_clone::AnyClone;

/// Combined trait for configurers that are both `AnyClone` and `SecurityConfigurer`.
///
/// Required because Rust cannot express `dyn AnyClone + SecurityConfigurer<O, B>` directly,
/// and we need to store configurers as trait objects that support both:
/// - `Any`-based downcasting (for `get_configurer::<C>()`)
/// - `SecurityConfigurer` trait method dispatch (for `init`/`configure` loops)
pub trait AnyCloneSecurityConfigurer<O, B>: AnyClone + SecurityConfigurer<O, B>
where
    B: SecurityBuilder<O>,
    O: Send + Sync,
{
}

impl<T, O, B> AnyCloneSecurityConfigurer<O, B> for T
where
    T: AnyClone + SecurityConfigurer<O, B>,
    B: SecurityBuilder<O>,
    O: Send + Sync,
{
}

// Manual Clone impl below — cannot derive because
// Box<dyn AnyCloneSecurityConfigurer<O, B>> does not implement Clone.
pub struct BaseConfiguredSecurityBuilder<O, B>
where
    B: SecurityBuilder<O>,
    O: Send + Sync,
{
    configurers: BTreeMap<TypeId, Vec<Box<dyn AnyCloneSecurityConfigurer<O, B>>>>,
    configurers_added_in_initializing: Vec<Box<dyn AnyCloneSecurityConfigurer<O, B>>>,
    pub(crate) shared_objects: HashMap<TypeId, Box<dyn AnyClone>>,
    pub(crate) build_state: BuildState,

    pub(crate) base_security_builder: BaseSecurityBuilder<O>,
    _marker: PhantomData<B>,
}

impl<O, B> BaseConfiguredSecurityBuilder<O, B>
where
    B: SecurityBuilder<O>,
    O: Send + Sync,
{
    pub fn new() -> Self {
        Self {
            configurers: Default::default(),
            configurers_added_in_initializing: Default::default(),
            shared_objects: Default::default(),
            build_state: BuildState::UNBUILT,
            base_security_builder: BaseSecurityBuilder::new(),
            _marker: PhantomData,
        }
    }

    pub fn set_shared_object<T>(&mut self, object: T)
    where
        T: AnyClone,
    {
        self.shared_objects
            .insert(std::any::TypeId::of::<T>(), Box::new(object));
    }

    pub fn get_shared_object<'a, T>(&'a self) -> Option<&'a T>
    where
        T: 'static,
    {
        self.shared_objects
            .get(&std::any::TypeId::of::<T>())
            .and_then(|obj| obj.as_ref().as_any().downcast_ref::<T>())
    }

    pub fn get_mut_shared_object<'a, T>(&'a mut self) -> Option<&'a mut T>
    where
        T: 'static,
    {
        self.shared_objects
            .get_mut(&std::any::TypeId::of::<T>())
            .and_then(|obj| {
                // obj: &mut Box<dyn AnyClone>
                // Deref to &mut dyn AnyClone, then upcast to &mut dyn Any (AnyClone: Any)
                let any: &mut dyn Any = &mut **obj;
                any.downcast_mut::<T>()
            })
    }

    pub fn with<C, F>(&mut self, mut configurer: C, mut f: F)
    where
        C: AnyClone + Clone + SecurityConfigurer<O, B> + 'static,
        F: FnMut(&mut C),
    {
        f(&mut configurer);

        self.add(configurer);
    }

    fn add<C>(&mut self, configurer: C)
    where
        C: AnyClone + Clone + SecurityConfigurer<O, B> + 'static,
    {
        if self.build_state.is_configured() {
            panic!("Cannot add configurer after build state is configured");
        }

        if self.build_state.is_initializing() {
            // Clone before moving: one copy goes to configurers_added_in_initializing
            // to have its init() called, the original stays in the configurers map.
            self.configurers_added_in_initializing
                .push(Box::new(configurer.clone()));
        }

        self.configurers
            .entry(std::any::TypeId::of::<C>())
            .or_insert(Vec::with_capacity(1))
            .push(Box::new(configurer));
    }

    pub fn configurer<C>(&self) -> Option<&C>
    where
        C: 'static,
        C: SecurityConfigurer<O, B>,
    {
        let configs = self.configurers.get(&std::any::TypeId::of::<C>())?;

        if configs.len() != 1 {
            panic!(
                "Only one configurer expected for type {}, but got len {}",
                std::any::type_name::<C>(),
                configs.len()
            )
        }

        // configs[0] is Box<dyn AnyCloneSecurityConfigurer<O, B>>
        // Deref to &mut dyn AnyCloneSC, upcast to &mut dyn Any, downcast to &mut C
        let any: &dyn Any = &*configs[0];
        any.downcast_ref::<C>()
    }

    pub fn configurer_mut<C>(&mut self) -> Option<&mut C>
    where
        C: 'static,
        C: SecurityConfigurer<O, B>,
    {
        let configs = self.configurers.get_mut(&std::any::TypeId::of::<C>())?;

        if configs.len() != 1 {
            panic!(
                "Only one configurer expected for type {}, but got len {}",
                std::any::type_name::<C>(),
                configs.len()
            )
        }

        // configs[0] is Box<dyn AnyCloneSecurityConfigurer<O, B>>
        // Deref to &mut dyn AnyCloneSC, upcast to &mut dyn Any, downcast to &mut C
        let any: &mut dyn Any = &mut *configs[0];
        any.downcast_mut::<C>()
    }

    pub fn remove_configurer<C>(&mut self) -> Option<C>
    where
        C: 'static,
        C: SecurityConfigurer<O, B>,
    {
        let key = TypeId::of::<C>();
        let mut configs = self.configurers.remove(&key)?;
        self.remove_from_configurers_added_in_initializing::<C>();

        if configs.len() != 1 {
            panic!("Only one configurer expected for type {}", type_name::<C>())
        }

        // configs[0]: Box<dyn AnyCloneSecurityConfigurer<O, B>>
        // Upcast to Box<dyn Any>, downcast to Box<C>, unbox
        let boxed: Box<dyn Any> = configs.remove(0);
        boxed.downcast::<C>().ok().map(|s| *s)
    }

    fn remove_from_configurers_added_in_initializing<C>(&mut self)
    where
        C: 'static,
        C: SecurityConfigurer<O, B>,
    {
        self.configurers_added_in_initializing.retain(|c| {
            // c: &Box<dyn AnyCloneSecurityConfigurer<O, B>>
            // Upcast to &dyn Any, check if it's C
            let any: &dyn Any = &**c;
            !any.is::<C>()
        });
    }

    pub fn remove_configurers<C>(&mut self) -> Vec<C>
    where
        C: AnyClone + 'static,
        C: SecurityConfigurer<O, B>,
    {
        let key = TypeId::of::<C>();
        let Some(configs) = self.configurers.remove(&key) else {
            return Default::default();
        };

        self.remove_from_configurers_added_in_initializing::<C>();

        configs
            .into_iter()
            .filter_map(|c| {
                let boxed: Box<dyn Any> = c;
                boxed.downcast::<C>().ok().map(|s| *s)
            })
            .collect()
    }

    /// Initialize all registered configurers.
    ///
    /// Takes `builder: &mut B` instead of `&mut self` to avoid overlapping borrows:
    /// the configurer references are extracted from `builder`'s configurers map via
    /// `std::mem::take`, processed while `builder` has no configurers, then merged back.
    pub(crate) fn init_configurers(builder: &mut B)
    where
        B: DerefMut<Target = Self> + SecurityBuilder<O>,
        O: 'static,
    {
        // 1. Take configurers map out of builder
        let mut configurers = std::mem::take(&mut builder.configurers);

        // 2. Init all configurers (refs are into the local `configurers`, not `builder`)
        {
            let configs: Vec<&mut dyn SecurityConfigurer<O, B>> = configurers
                .values_mut()
                .flatten()
                .map(|boxed| &mut **boxed as &mut dyn SecurityConfigurer<O, B>)
                .collect();

            for configurer in configs {
                configurer.init(builder);
            }
        }

        // 3. Handle configurers added during initialization
        while !builder.configurers_added_in_initializing.is_empty() {
            let to_init = std::mem::take(&mut builder.configurers_added_in_initializing);

            for boxed in to_init {
                let mut sc: Box<dyn SecurityConfigurer<O, B>> = boxed;
                sc.init(builder);
            }
        }

        // 4. Merge: new configurers may have been added to builder.configurers during init
        for (key, mut new_configs) in std::mem::take(&mut builder.configurers) {
            configurers.entry(key).or_default().append(&mut new_configs);
        }
        builder.configurers = configurers;
    }

    /// Configure all registered configurers.
    ///
    /// Same pattern as `init_configurers`: takes `builder: &mut B` and uses
    /// `std::mem::take` to avoid overlapping borrows.
    pub(crate) fn configure_configurers(builder: &mut B)
    where
        B: DerefMut<Target = Self> + SecurityBuilder<O>,
        O: 'static,
    {
        let mut configurers = std::mem::take(&mut builder.configurers);

        {
            let configs: Vec<&mut dyn SecurityConfigurer<O, B>> = configurers
                .values_mut()
                .flatten()
                .map(|boxed| &mut **boxed as &mut dyn SecurityConfigurer<O, B>)
                .collect();

            for configurer in configs {
                configurer.configure(builder);
            }
        }

        builder.configurers = configurers;
    }

    fn is_unbuilt(&self) -> bool {
        self.build_state == BuildState::UNBUILT
    }
}

// Manual Clone impl: cannot clone `Box<dyn AnyCloneSecurityConfigurer<O, B>>`,
// so configurers are re-initialized as empty in the clone.
impl<O, B> Clone for BaseConfiguredSecurityBuilder<O, B>
where
    B: SecurityBuilder<O>,
    O: Send + Sync,
{
    fn clone(&self) -> Self {
        Self {
            configurers: BTreeMap::new(),
            configurers_added_in_initializing: Vec::new(),
            shared_objects: self.shared_objects.clone(),
            build_state: self.build_state,
            base_security_builder: self.base_security_builder.clone(),
            _marker: PhantomData,
        }
    }
}

impl<O, B> Default for BaseConfiguredSecurityBuilder<O, B>
where
    B: SecurityBuilder<O>,
    O: Send + Sync,
{
    fn default() -> Self {
        Self {
            configurers: BTreeMap::new(),
            configurers_added_in_initializing: Default::default(),
            shared_objects: Default::default(),
            build_state: BuildState::UNBUILT,

            base_security_builder: Default::default(),
            _marker: PhantomData,
        }
    }
}

impl<O, B> Deref for BaseConfiguredSecurityBuilder<O, B>
where
    B: SecurityBuilder<O>,
    O: Send + Sync,
{
    type Target = BaseSecurityBuilder<O>;

    fn deref(&self) -> &Self::Target {
        &self.base_security_builder
    }
}

impl<O, B> DerefMut for BaseConfiguredSecurityBuilder<O, B>
where
    B: SecurityBuilder<O>,
    O: Send + Sync,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_security_builder
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuildState {
    #[default]
    UNBUILT,
    INITIALIZING,
    CONFIGURING,
    BUILDING,
    BUILT,
}

impl BuildState {
    pub fn order(&self) -> i32 {
        match self {
            BuildState::UNBUILT => 0,
            BuildState::INITIALIZING => 1,
            BuildState::CONFIGURING => 2,
            BuildState::BUILDING => 3,
            BuildState::BUILT => 4,
        }
    }

    pub fn is_initializing(&self) -> bool {
        *self == BuildState::INITIALIZING
    }

    pub fn is_configured(&self) -> bool {
        self.order() >= BuildState::CONFIGURING.order()
    }
}

pub trait BaseConfiguredSecurityBuilderExt<O, B>
where
    B: SecurityBuilder<O>,
    O: Send + Sync,
{
    fn before_init(&mut self);

    fn before_configure(&mut self);

    fn perform_build(&mut self) -> O;
}
