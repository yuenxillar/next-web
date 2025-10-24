use crate::core::{
    cache::{cache_manager::CacheManager, cache_manager_aware::CacheManagerAware},
    event::{
        event_bus::EventBus, event_bus_aware::EventBusAware,
        support::default_event_bus::DefaultEventBus,
    },
    util::destroyable::Destroyable,
};

#[derive(Clone)]
pub struct CachingSecurityManager<C, B = DefaultEventBus> {
    cache_manager: Option<C>,
    event_bus: Option<B>,
}

impl<C, B> CachingSecurityManager<C, B>
where
    C: CacheManager,
    B: EventBus,
{
    pub fn get_cache_manager(&self) -> Option<&C> {
        self.cache_manager.as_ref()
    }

    pub fn get_event_bus(&self) -> Option<&B> {
        self.event_bus.as_ref()
    }
}

impl<C, B> CachingSecurityManager<C, B>
where
    C: CacheManager + EventBusAware<B>,
    B: EventBus + Clone,
{
    pub fn after_cache_manager_set(&mut self) {
        self.apply_event_bus_to_cache_manager();
    }

    pub fn apply_event_bus_to_cache_manager(&mut self) {
        let event_bus = match self.event_bus.as_ref() {
            Some(event_bus) => event_bus,
            None => return,
        };

        let cache_manager = match self.cache_manager.as_mut() {
            Some(cache_manager) => cache_manager,
            None => return,
        };
        cache_manager.set_event_bus(event_bus.clone());
    }

    pub fn after_event_bus_set(&mut self) {
        self.apply_event_bus_to_cache_manager();
    }
}

impl<C> CacheManagerAware<C> for CachingSecurityManager<C>
where
    C: CacheManager,
    C: EventBusAware<DefaultEventBus>,
{
    fn set_cache_manager(&mut self, cache_manager: C) {
        self.cache_manager = Some(cache_manager);
        self.after_cache_manager_set();
    }
}

impl<C, B> Destroyable for CachingSecurityManager<C, B>
where
    C: CacheManager,
    B: EventBus + Default,
    B: Destroyable
{
    fn destroy(mut self) {
        self.cache_manager = None;

        if let Some(event_bus) = self.event_bus.take() {
            event_bus.destroy();
        }

        self.event_bus = Some(Default::default());
    }
}

impl<C, B> EventBusAware<B> for CachingSecurityManager<C, B>
where
    C: CacheManager + EventBusAware<B>,
    B: EventBus + Clone,
{
    fn set_event_bus(&mut self, event_bus: B) {
        self.event_bus = Some(event_bus);
        self.after_event_bus_set();
    }
}

impl<C, B> Default for CachingSecurityManager<C, B>
where
    C: CacheManager + EventBusAware<B>,
    B: Default + Clone + EventBus
{
    fn default() -> Self {
        let mut manager = Self {
            cache_manager: Default::default(),
            event_bus: Default::default(),
        };

        manager.set_event_bus(Default::default());

        manager
    }
}
