use crate::core::object::Object;
use next_web_core::anys::any_map::AnyMap;
use std::{any::Any, collections::HashMap, fmt::Display};

pub trait CacheManager: Send + Sync
where
    Self: Display,
    Self: Any
{
    fn get_cache(&self, name: &str) -> Option<&HashMap<String, Object>>;
}

pub trait CacheManagerSupport: Send + Sync
where
    Self: Display,
{
    fn get_cache_by_type<T>(&self, name: &str) -> Option<&AnyMap<T>>;
}
