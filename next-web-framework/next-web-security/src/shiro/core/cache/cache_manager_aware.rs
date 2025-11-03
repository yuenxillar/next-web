use crate::core::cache::cache_manager::CacheManager;


pub trait CacheManagerAware<T: CacheManager> {
    fn set_cache_manager(&mut self, cache_manager: T);
}