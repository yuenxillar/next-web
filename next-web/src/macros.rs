pub use inventory::submit;

pub use next_web_macros::idempotency;
pub use next_web_macros::order;

pub mod bind {

    pub use next_web_macros::{
        any_mapping, delete_mapping, get_mapping, patch_mapping, post_mapping, put_mapping,
        request_mapping,
    };
    pub use next_web_macros::{singleowner, singleton, transient};
}

pub mod autoconfigure {
    pub use next_web_macros::auto_configuration;
    pub use next_web_macros::configuration;
    pub use next_web_macros::configuration_properties;
}

pub mod data {
    pub use next_web_macros::Desensitize;
    pub use next_web_macros::{Builder, FieldName, GetSet, RequiredArgsConstructor};
}

pub mod event {
    pub use next_web_macros::event_listener;
}

#[cfg(feature = "enable-scheduling")]
pub use next_web_macros::scheduled;

#[cfg(feature = "enable-open-api")]
pub use next_web_macros::open_api;

pub mod application {
    pub use next_web_macros::next_application;
}

#[cfg(feature = "enable-web-security")]
pub mod security {
    pub use next_web_macros::pre_authorize;
}

#[cfg(feature = "enable-retry")]
pub use next_web_macros::retryable;

#[cfg(feature = "enable-i18n")]
pub mod i18n {}
