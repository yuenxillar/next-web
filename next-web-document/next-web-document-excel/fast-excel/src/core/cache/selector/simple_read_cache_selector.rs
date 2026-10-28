use crate::core::cache::selector::read_cache_selector::ReadCacheSelector;

pub struct SimpleReadCacheSelector {
    maxUseMapCacheSize: Option<u64>,
    maxCacheActivateBatchCount: Option<u64>,
}

impl SimpleReadCacheSelector {
    const B2M: u32 = 1000000;
    const DEFAULT_MAX_USE_MAP_CACHE_SIZE: i32 = 5;
    const DEFAULT_MAX_EHCACHE_ACTIVATE_BATCH_COUNT: i32 = 20;
}

impl ReadCacheSelector for SimpleReadCacheSelector {}

impl Default for SimpleReadCacheSelector {
    fn default() -> Self {
        SimpleReadCacheSelector {
            maxUseMapCacheSize: Some(100 * Self::B2M as u64),
            maxCacheActivateBatchCount: Some(1000),
        }
    }
}
