use next_web_core::async_trait;

use crate::core::{authc::{authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken}, cache::{cache_manager_aware::CacheManagerAware, default_cache_manager::DefaultCacheManager}, event::{event_bus_aware::EventBusAware, support::default_event_bus::DefaultEventBus}, realm::Realm};


#[derive(Clone)]
pub struct SimpleAccountRealm {

}

#[async_trait]
impl Realm for SimpleAccountRealm {
    fn get_name(&self) ->  &str {
        todo!()
    }

    fn supports(&self,authentication_token: &dyn AuthenticationToken) -> bool {
        todo!()
    }

    async fn get_authentication_info(
        &self,
        token: &dyn AuthenticationToken,
    ) -> Option<Box<dyn AuthenticationInfo>> {
        None
    }
}

impl CacheManagerAware<DefaultCacheManager> for SimpleAccountRealm {
    fn set_cache_manager(&mut self, cache_manager: DefaultCacheManager) {
        todo!()
    }
}

impl EventBusAware<DefaultEventBus> for SimpleAccountRealm {
    fn set_event_bus(&mut self, event_bus: DefaultEventBus) {
        todo!()
    }
}


impl Default for SimpleAccountRealm {
    fn default() -> Self {
        Self {  }
    }
}