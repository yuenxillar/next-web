use std::any::Any;

use next_web_core::traits::required::Required;

use crate::core::{
    authc::{
        authentication_error::AuthenticationError, authentication_info::AuthenticationInfo,
        authentication_token::AuthenticationToken, authenticator::Authenticator,
        pam::modular_realm_authenticator::ModularRealmAuthenticator,
    },
    cache::{cache_manager::CacheManager, cache_manager_aware::CacheManagerAware, default_cache_manager::DefaultCacheManager},
    event::{event_bus::EventBus, event_bus_aware::EventBusAware, support::default_event_bus::DefaultEventBus},
    mgt::realm_security_manager::RealmSecurityManager,
    realm::{simple_account_realm::SimpleAccountRealm, Realm},
    util::destroyable::Destroyable,
};

#[derive(Clone)]
pub struct AuthenticatingSecurityManager<
    T = ModularRealmAuthenticator,
    R = SimpleAccountRealm,
    C = DefaultCacheManager,
    B = DefaultEventBus,
> {
    authenticator: T,
    realm_security_manager: RealmSecurityManager<R, C, B>,
}

impl<T> AuthenticatingSecurityManager<T>
where
    T: Authenticator,
{
    pub fn get_authenticator(&self) -> &T {
        & self.authenticator
    }

    pub fn set_authenticator(&mut self, authenticator: T) {
        self.authenticator = authenticator;
    }
}

impl<T, R, C, B> Authenticator for AuthenticatingSecurityManager<T, R, C, B>
where
    T: Authenticator,
    R: Send + Sync,
    C: Send + Sync,
    B: Send + Sync,
{
    fn authenticate(
        &self,
        authentication_token: &dyn AuthenticationToken,
    ) -> Result<&dyn AuthenticationInfo, AuthenticationError> {
        self.authenticator.authenticate(authentication_token)
    }
}


impl<T, R, C, B> AuthenticatingSecurityManager<T, R, C, B>
where
    T: Authenticator + Any,
    R: Realm + CacheManagerAware<C> + EventBusAware<B>,
    R: Clone,
    C: CacheManager + Clone,
    B: EventBus + Clone,
{
    pub fn after_realms_set(&mut self) {
        self.realm_security_manager.after_realms_set();
        if let Some(authenticator) =
            (&mut self.authenticator as &mut dyn Any).downcast_mut::<ModularRealmAuthenticator>()
        {
            let realms = self.realm_security_manager.get_realms().clone();
            authenticator.set_realms(realms);
        }
    }
}

impl<T, R, C, B> Destroyable for AuthenticatingSecurityManager<T, R, C, B>
where
    T: Authenticator + Destroyable,
    R: Realm,
    C: CacheManager,
    B: Default + EventBus + Destroyable
{
    fn destroy(self) {
        self.authenticator.destroy();
        self.realm_security_manager.destroy();
    }
}


impl<T, R, C, B> Required<RealmSecurityManager<R, C, B>> for AuthenticatingSecurityManager<T, R, C, B>
where
    T: Authenticator,
    R: Realm + CacheManagerAware<C> + EventBusAware<B>,
    R: Clone,
    C: CacheManager + Clone,
    B: EventBus + Clone,
{
    fn get_object(&self) -> &RealmSecurityManager<R, C, B> {
        &self.realm_security_manager
    }

    fn get_mut_object(&mut self) -> &mut RealmSecurityManager<R, C, B> {
        &mut self.realm_security_manager
    }
}

impl<T, R, C, B> Default for AuthenticatingSecurityManager<T, R, C, B>
where
    T: Default,
    C: Default + CacheManager + EventBusAware<B>,
    R: Default + Realm,
    B: Default + Clone + EventBus,
{
    fn default() -> Self {
        Self {
            authenticator: Default::default(),
            realm_security_manager: Default::default(),
        }
    }
}
