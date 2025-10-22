use std::sync::Arc;

use crate::core::cache::cache_manager::CacheManager;


pub trait CacheManagerAware {
    fn set_cache_manager(&mut self, cache_manager: Arc<dyn CacheManager>);
}