pub mod handler_autoregister;

#[cfg(feature = "enable-scheduling")]
pub mod scheduler_autoregister;

#[cfg(feature = "i18n")]
pub mod message_source_service_autoregister;

pub mod register_single;
