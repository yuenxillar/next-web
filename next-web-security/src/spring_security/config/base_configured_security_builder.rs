use std::any::{type_name, Any, TypeId};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::{collections::BTreeMap, marker::PhantomData};

use crate::config::{
    base_security_builder::BaseSecurityBuilder, security_builder::SecurityBuilder,
};
use next_web_core::traits::any_clone::AnyClone;

#[derive(Clone)]
pub struct BaseConfiguredSecurityBuilder<O, B>
where
    B: SecurityBuilder<O>,
    O: Send + Sync,
{
    configurers: BTreeMap<TypeId, Vec<Box<dyn AnyClone>>>,
    configurers_added_in_initializing: Vec<Box<dyn AnyClone>>,
    pub(crate) shared_objects: HashMap<TypeId, Box<dyn AnyClone>>,
    allow_configurers_of_same_type: bool,
    pub(crate) build_state: BuildState,

    pub(crate) base_security_builder: BaseSecurityBuilder<O>,
    _marker: PhantomData<(O, B)>,
}

impl<O, B> BaseConfiguredSecurityBuilder<O, B>
where
    B: SecurityBuilder<O>,
    O: Send + Sync,
{
    pub fn new(allow_configurers_of_same_type: bool) -> Self {
        Self {
            configurers: Default::default(),
            configurers_added_in_initializing: Default::default(),
            shared_objects: Default::default(),
            build_state: BuildState::UNBUILT,
            base_security_builder: BaseSecurityBuilder::new(),
            allow_configurers_of_same_type,
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
            .and_then(|obj| (obj.as_mut() as &mut dyn Any).downcast_mut::<T>())
    }

    pub fn get_configurer<C>(&mut self) -> Option<&mut C>
    // where
    //     C: SecurityConfigurer<O, B>,
    where
        C: 'static,
    {
        let configs = match self.configurers.get_mut(&std::any::TypeId::of::<C>()) {
            Some(configs) => configs,
            None => return None,
        };

        if configs.len() == 1 {
            panic!(
                "Only one configurer expected for type {}, but got len {}",
                std::any::type_name::<C>(),
                configs.len()
            )
        }

        configs
            .get_mut(0)
            .and_then(|c| (c as &mut dyn Any).downcast_mut::<C>())
    }

    pub fn remove_configurer<C>(&mut self) -> Option<C>
    where
        C: AnyClone + 'static,
    {
        let key = TypeId::of::<C>();
        let mut configs = self.configurers.remove(&key)?;
        if configs.len() == 1 {
            panic!("Only one configurer expected for type {}", type_name::<C>())
        }

        configs
            .remove(0)
            .into_any()
            .downcast::<C>()
            .map(|s| *s)
            .ok()
    }

    pub fn with<C, F>(&mut self, mut configurer: C, mut f: F)
    where
        C: AnyClone + 'static,
        F: FnMut(&mut C),
    {
        f(&mut configurer);

        self.add(configurer);
    }

    fn add<C>(&mut self, configurer: C)
    where
        C: AnyClone + 'static,
    {
        if self.build_state.is_configured() {
            panic!("Cannot add configurer after build state is configured");
        }

        self.configurers
            .entry(std::any::TypeId::of::<C>())
            .or_insert(Vec::with_capacity(1))
            .push(Box::new(configurer));

        if self.build_state.is_initializing() {
            // self.configurers_added_in_initializing
            //     .push();
            todo!()
        }
    }

    pub(crate) fn configure(&mut self) {}

    pub(crate) fn init(&mut self) {}
}

// impl<O, B> BaseSecurityBuilderExt<O> for BaseConfiguredSecurityBuilder<O, B>
// where
//     B: SecurityBuilder<O>,
//     O: Send + Sync,
//     O: BaseConfiguredSecurityBuilderExt<O, B>,
// {
//     fn do_build(&mut self) -> O {
//         self.build_state = BuildState::INITIALIZING;
//         self.before_init();
//         self.init();
//         self.build_state = BuildState::CONFIGURING;
//         self.base_security_builder.before_configure();
//         self.configure();
//         self.build_state = BuildState::BUILDING;
//         let result = self.perform_build();
//         self.build_state = BuildState::BUILT;

//         result
//     }
// }

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
            allow_configurers_of_same_type: Default::default(),
            build_state: BuildState::UNBUILT,

            base_security_builder: Default::default(),
            _marker: PhantomData::default(),
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
