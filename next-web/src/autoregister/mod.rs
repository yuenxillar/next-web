pub mod application_event_autoregister;
pub mod http_handler_autoregister;

/// Registration of the state machines of an application.
#[cfg(feature = "enable-state-machine")]
pub mod state_machine_autoregister;
