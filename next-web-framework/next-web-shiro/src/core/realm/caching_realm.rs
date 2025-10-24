use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use tracing::{debug, trace};

use crate::core::{
    authc::{
        authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken,
        logout_aware::LogoutAware,
    },
    cache::{cache_manager::CacheManager, cache_manager_aware::CacheManagerAware},
    object::Object,
    subject::principal_collection::PrincipalCollection,
    util::nameable::Nameable,
};

#[derive(Clone)]
pub struct CachingRealm {
    name: String,
    cache_manager: Option<Arc<dyn CacheManager>>,
    caching_enabled: bool,
}

impl CachingRealm {
    const INSTANCE_COUNT: AtomicU32 = AtomicU32::new(0);

    pub fn set_caching_enabled(&mut self, enabled: bool) {
        self.caching_enabled = enabled;
    }

    pub fn is_caching_enabled(&self) -> bool {
        self.caching_enabled
    }

    pub fn is_empty(&self, principals: &dyn PrincipalCollection) -> bool {
        principals.is_empty()
    }

    pub fn get_cache_manager(&self) -> Option<&dyn CacheManager> {
        self.cache_manager.as_deref()
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    // 返回值 用于决定实现者的逻辑执行
    pub fn clear_cache(&mut self, principals: &dyn PrincipalCollection) -> bool{
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

impl Nameable for CachingRealm {
    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }
}

impl CacheManagerAware for CachingRealm {
    fn set_cache_manager(&mut self, cache_manager: Arc<dyn CacheManager>) {
        self.cache_manager = Some(cache_manager);
        // self.after_cache_manager_set();
    }
}


pub trait CachingRealmSupport: Send + Sync
{
    fn after_cache_manager_set(&mut self);
    fn do_clear_cache(&mut self, principals: &dyn PrincipalCollection);
}