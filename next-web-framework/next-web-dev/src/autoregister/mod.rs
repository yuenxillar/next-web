pub mod handler_autoregister;
pub mod register_single;


#[cfg(feature = "enable-scheduling")]
pub mod scheduler_autoregister;

#[cfg(feature = "enable-i18n")]
pub mod message_source_service_autoregister;

