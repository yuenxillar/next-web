/// Auto-registration entry points for the Redis starter.
pub mod redis_auto_configuration;
pub mod redis_cluster_properties;
pub mod redis_properties;
pub mod redis_sentinel_properties;
pub mod redis_url;
#[cfg(feature = "distributed-lock")]
pub mod redis_lock_properties;


pub use redis_url::RedisUrl;
