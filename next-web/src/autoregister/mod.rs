pub mod application_event_autoregister;
pub mod default_autoregister;
pub mod http_handler_autoregister;

#[cfg(feature = "enable-scheduling")]
pub mod scheduler_autoregister;

#[cfg(feature = "enable-i18n")]
pub mod message_source_service_autoregister;
