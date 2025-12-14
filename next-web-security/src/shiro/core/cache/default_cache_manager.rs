use std::fmt::Display;

use crate::core::{cache::cache_manager::CacheManager, event::{event_bus_aware::EventBusAware, support::default_event_bus::DefaultEventBus}};


#[derive(Clone)]
pub struct DefaultCacheManager {}


impl CacheManager for DefaultCacheManager {
    fn get_cache(&self, name: &str) -> Option<&std::collections::HashMap<String, crate::core::util::object::Object>> {
        todo!()
    }
}



impl EventBusAware<DefaultEventBus> for DefaultCacheManager {
    fn set_event_bus(&mut self, event_bus: DefaultEventBus) {
        todo!()
    }
}

impl Display for DefaultCacheManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DefaultCacheManager")
    }
}

impl Default for DefaultCacheManager {
    fn default() -> Self {
        Self {  }
    }
}
