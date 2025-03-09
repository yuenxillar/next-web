#[cfg(feature = "mongodb_enabled")]
pub mod mongodb_autoregister;

#[cfg(feature = "job_scheduler")]
pub mod job_scheduler_autoregister;

#[cfg(feature = "minio_enabled")]
pub mod minio_autoregister;

#[cfg(feature = "redis_enabled")]
pub mod redis_autoregister;

#[cfg(feature = "rabbitmq_enabled")]
pub mod rabbitmq_autoregister;

#[cfg(feature = "mqtt_enabled")]
pub mod mqtt_autoregister;

#[cfg(feature = "database_enabled")]
pub mod database_autoregister;


pub mod register_single;
pub mod auto_register;

