use std::sync::atomic::{AtomicU32, Ordering};

use next_web_core::async_trait;
use tracing::trace;

use crate::core::{
    authc::{authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken},
    cache::{
        cache_manager::CacheManager, cache_manager_aware::CacheManagerAware,
        default_cache_manager::DefaultCacheManager,
    },
    realm::Realm,
    subject::principal_collection::PrincipalCollection,
    util::object::Object,
};

#[derive(Clone)]
pub struct CachingRealm<T = DefaultCacheManager> {
    name: String,
    cache_manager: Option<T>,
    caching_enabled: bool,
}

impl<T> CachingRealm<T> {
    const INSTANCE_COUNT: AtomicU32 = AtomicU32::new(0);

    pub fn set_caching_enabled(&mut self, enabled: bool) {
        self.caching_enabled = enabled;
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    pub fn is_caching_enabled(&self) -> bool {
        self.caching_enabled
    }

    pub fn is_empty(&self, principals: &dyn PrincipalCollection) -> bool {
        principals.is_empty()
    }

    pub fn get_cache_manager(&self) -> Option<&T> {
        self.cache_manager.as_ref()
    }

    // 返回值 用于决定实现者的逻辑执行
    pub fn clear_cache(&self, principals: &dyn PrincipalCollection) -> bool {
        if !principals.is_empty() {
            trace!(
                "Cleared cache entries for account with principals [{}]",
                principals.to_string()
            );
            return true;
        }

        false
    }

    pub fn get_available_principal(&self, principals: &dyn PrincipalCollection) -> Object {
        // if !principals.is_empty() {

        // }

        todo!()
    }
}

#[async_trait]
impl<T> Realm for CachingRealm<T>
where
    T: Send + Sync,
{
    fn get_name(&self) -> &str {
        &self.name
    }

    fn supports(&self, authentication_token: &dyn AuthenticationToken) -> bool {
        todo!()
    }

    async fn get_authentication_info(
        &self,
        token: &dyn AuthenticationToken,
    ) -> Option<Box<dyn AuthenticationInfo>> {
        todo!()
    }
}

impl<T> CacheManagerAware<T> for CachingRealm<T>
where
    T: CacheManager,
{
    fn set_cache_manager(&mut self, cache_manager: T) {
        self.cache_manager = Some(cache_manager);
        // self.after_cache_manager_set();
    }
}

impl Default for CachingRealm {
    fn default() -> Self {
        Self {
            name: format!(
                "{}_{}",
                std::any::type_name::<Self>(),
                Self::INSTANCE_COUNT.load(Ordering::Relaxed)
            ),
            cache_manager: None,
            caching_enabled: true,
        }
    }
}

pub trait CachingRealmSupport: Send + Sync {
    fn after_cache_manager_set(&mut self);
    fn do_clear_cache(&self, principals: &dyn PrincipalCollection);
}
