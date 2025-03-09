#[cfg(feature = "mongodb_enabled")]
pub mod mongodb_manager;

#[cfg(feature = "mqtt_enabled")]
pub mod mqtt_manager;

#[cfg(feature = "user_security")]
pub mod user_authorization_manager;

#[cfg(feature = "minio_enabled")]
pub mod minio_manager;

#[cfg(feature = "database_enabled")]
pub mod database_manager;

#[cfg(feature = "redis_enabled")]
pub mod redis_manager;

#[cfg(feature = "job_scheduler")]
pub mod job_scheduler_manager;

#[cfg(feature = "rabbitmq_enabled")]
pub mod rabbitmq_manager;
