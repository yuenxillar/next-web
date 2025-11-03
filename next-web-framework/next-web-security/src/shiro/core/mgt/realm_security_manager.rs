use std::ops::{Deref, DerefMut};

use next_web_core::traits::required::Required;

use crate::core::{
    cache::{
        cache_manager::CacheManager, cache_manager_aware::CacheManagerAware,
        default_cache_manager::DefaultCacheManager,
    },
    event::{
        event_bus::EventBus, event_bus_aware::EventBusAware,
        support::default_event_bus::DefaultEventBus,
    },
    mgt::caching_security_manager::CachingSecurityManager,
    realm::Realm,
    util::destroyable::Destroyable,
};

#[derive(Clone)]
pub struct RealmSecurityManager<R, C = DefaultCacheManager, B = DefaultEventBus> {
    realms: Vec<R>,
    caching_security_manager: CachingSecurityManager<C, B>,
}

impl<R: Realm> RealmSecurityManager<R> {
    pub fn set_realm(&mut self, realm: R) {
        self.set_realms(vec![realm]);
    }

    pub fn set_realms(&mut self, realms: Vec<R>) {
        assert!(
            realms.len() > 0,
            "Realms collection argument cannot be empty."
        );
        self.realms = realms;
    }
}

impl<R, C, B> RealmSecurityManager<R, C, B>
where
    R: Realm + CacheManagerAware<C> + EventBusAware<B>,
    C: CacheManager + Clone,
    B: EventBus + Clone,
{
    pub fn get_realms(&self) -> &Vec<R> {
        &self.realms
    }

    pub fn get_mut_realms(&mut self) -> &mut Vec<R> {
        &mut self.realms
    }

    pub fn after_realms_set(&mut self) {
        self.apply_cache_manager_to_realms();
        self.apply_event_bus_to_realms();
    }

    pub fn apply_cache_manager_to_realms(&mut self) {
        let cache_manager = match self.get_owned_cache_manager() {
            Some(cache_manager) => cache_manager,
            None => return,
        };

        let realms = self.get_mut_realms();

        if !realms.is_empty() {
            for realm in realms {
                realm.set_cache_manager(cache_manager.to_owned());
            }
        }
    }

    pub fn apply_event_bus_to_realms(&mut self) {
        let event_bus = match self.get_event_bus() {
            Some(event_bus) => event_bus.clone(),
            None => return,
        };

        let realms = self.get_realms();
        if realms.is_empty() {
            return;
        }

        for realm in self.get_mut_realms() {
            realm.set_event_bus(event_bus.clone());
        }
    }
}

impl<R, C> RealmSecurityManager<R, C>
where
    R: Realm + CacheManagerAware<C> + EventBusAware<DefaultEventBus>,
    C: CacheManager + Clone + EventBusAware<DefaultEventBus>,
{
    pub fn after_cache_manager_set(&mut self) {
        self.caching_security_manager.after_cache_manager_set();
        self.apply_cache_manager_to_realms();
    }

    pub fn after_event_bus_set(&mut self) {
        self.caching_security_manager.after_event_bus_set();
        self.apply_event_bus_to_realms();
    }
}

impl<R, C, B> Destroyable for RealmSecurityManager<R, C, B>
where
    R: Realm,
    C: CacheManager,
    B: Default + EventBus + Destroyable,
{
    fn destroy(mut self) {
        self.realms.clear();
        self.caching_security_manager.destroy();
    }
}

impl<R, C, B> Required<CachingSecurityManager<C, B>> for RealmSecurityManager<R, C, B>
where
    R: Realm,
    C: CacheManager,
    B: EventBus,
{
    fn get_object(&self) -> &CachingSecurityManager<C, B> {
        &self.caching_security_manager
    }

    fn get_mut_object(&mut self) -> &mut CachingSecurityManager<C, B> {
        &mut self.caching_security_manager
    }
}

impl<R, C, B> Deref for RealmSecurityManager<R, C, B>
where
    R: Realm,
    C: CacheManager,
    B: EventBus,
{
    type Target = CachingSecurityManager<C, B>;

    fn deref(&self) -> &Self::Target {
        &self.caching_security_manager
    }
}

impl<R, C, B> DerefMut for RealmSecurityManager<R, C, B>
where
    R: Realm,
    C: CacheManager,
    B: EventBus,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.caching_security_manager
    }
}

impl<R, C, B> Default for RealmSecurityManager<R, C, B>
where
    R: Realm + Default,
    C: Default + CacheManager + EventBusAware<B>,
    B: Default + Clone + EventBus,
{
    fn default() -> Self {
        Self {
            realms: Default::default(),
            caching_security_manager: Default::default(),
        }
    }
}
