
pub mod application_properties;
pub mod data_properties;

pub mod http_properties;
pub mod logger_properties;
pub mod message_source_properties;
pub mod next_properties;
pub mod security_properties;
pub mod server_properties;
pub mod user_authorization_options_properties;

#[cfg(feature = "database_enabled")]
pub mod datasource_properties;

#[cfg(feature = "minio_enabled")]
pub mod minio_properties;

#[cfg(feature = "redis_enabled")]
pub mod redis_properties;

#[cfg(feature = "mongodb_enabled")]
pub mod mongodb_properties;

