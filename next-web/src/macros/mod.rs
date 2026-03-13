pub use inventory::submit;

pub use next_web_macros::idempotency;

pub mod bind {

    pub use next_web_macros::{
        any_mapping, delete_mapping, get_mapping, patch_mapping, post_mapping, put_mapping,
        request_mapping,
    };
    pub use rudi_dev::{singleowner, singleton, transient};

    pub use next_web_macros::properties;
}

pub mod data {
    pub use next_web_macros::Desensitized;
    pub use next_web_macros::{Builder, FieldName, GetSet, RequiredArgsConstructor};
}

pub mod event {
    pub use next_web_macros::event_listener;
}

#[cfg(feature = "enable-scheduling")]
pub use next_web_macros::scheduled;

#[cfg(feature = "enable-api-doc")]
pub use next_web_macros::api_doc;
