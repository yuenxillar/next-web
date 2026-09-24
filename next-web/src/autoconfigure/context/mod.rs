// #[cfg(feature = "enable-i18n")]
pub mod message_source_properties;
// #[cfg(feature = "enable-i18n")]
pub mod message_source_auto_configuration;

mod property_placeholder_auto_configuration;

pub use property_placeholder_auto_configuration::PropertyPlaceholderAutoConfiguration;
