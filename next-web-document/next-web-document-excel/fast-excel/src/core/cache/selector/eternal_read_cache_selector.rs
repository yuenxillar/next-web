use crate::core::cache::{read_cache::ReadCache, selector::read_cache_selector::ReadCacheSelector};

pub struct EternalReadCacheSelector<T> {
    read_cache: T,
}

impl<T: ReadCache> EternalReadCacheSelector<T> {
    pub fn new(read_cache: T) -> Self {
        EternalReadCacheSelector { read_cache }
    }
}

impl<T> ReadCacheSelector for EternalReadCacheSelector<T> where T: ReadCache {}
