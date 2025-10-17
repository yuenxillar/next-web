use std::sync::Arc;
use std::{collections::BTreeMap, marker::PhantomData};

use crate::config::object_post_processor::ObjectPostProcessor;
use crate::config::security_configurer::SecurityConfigurer;
use crate::config::{
    abstract_security_builder::AbstractSecurityBuilder, security_builder::SecurityBuilder,
};
use next_web_core::anys::any_map::AnyMap;
use next_web_core::traits::required::Required;

#[derive(Clone)]
pub struct AbstractConfiguredSecurityBuilder<O, B>
where
    B: SecurityBuilder<O>,
    O: Send + Sync,
    Self: Required<AbstractSecurityBuilder<O>>,
{
    configurers: BTreeMap<String, Vec<Arc<dyn SecurityConfigurer<O, B>>>>,
    configurers_added_in_initializing: Vec<Arc<dyn SecurityConfigurer<O, B>>>,
    shared_objects: AnyMap,
    allow_configurers_of_same_type: bool,
    build_state: BuildState,

    abstract_security_builder: AbstractSecurityBuilder<O>,
    _marker: PhantomData<(O, B)>,
}

impl<O, B> AbstractConfiguredSecurityBuilder<O, B>
where
    B: SecurityBuilder<O>,
    O: Send + Sync,
{
    pub fn new(
        allow_configurers_of_same_type: bool,
    ) -> Self {
        Self {
            configurers: Default::default(),
            configurers_added_in_initializing: Default::default(),
            shared_objects: Default::default(),
            build_state: BuildState::UNBUILT,
            abstract_security_builder: AbstractSecurityBuilder::new(),
            allow_configurers_of_same_type,
            _marker: PhantomData
        }
    }
}

impl<O, B> Required<AbstractSecurityBuilder<O>> for AbstractConfiguredSecurityBuilder<O, B>
where
    B: SecurityBuilder<O>,
    O: Send + Sync,
{
    fn get_object(&self) -> &AbstractSecurityBuilder<O> {
        &self.abstract_security_builder
    }

    fn get_object_mut(&mut self) -> &mut AbstractSecurityBuilder<O> {
        &mut self.abstract_security_builder
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
