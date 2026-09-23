pub mod property_decryptor;

mod default_properties_property_source;
mod properties_loader;
mod properties_property_source_loader;
mod property_source_info;
mod property_source_loader;
mod yaml_loader;
mod yaml_property_source_loader;

mod decrypt_properties_environment_post_processor;

pub use decrypt_properties_environment_post_processor::{
    DecryptPropertiesEnvironmentPostProcessor, DECRYPT_PASSWORD_ARGUMENT, DECRYPT_PASSWORD_PROPERTY,
};
pub use default_properties_property_source::{
    DefaultPropertiesPropertySource, DEFAULT_PROPERTIES_PROPERTY_SOURCE_NAME,
};
pub use next_web_core::env::MapPropertySource;
pub use properties_property_source_loader::PropertiesPropertySourceLoader;
pub use property_decryptor::{PropertyDecryptor, SECURE_PROPERTY_PREFIX};
pub use property_source_info::PropertySourceInfo;
pub use property_source_loader::PropertySourceLoader;
pub use yaml_property_source_loader::YamlPropertySourceLoader;

pub(crate) use properties_loader::PropertiesLoader;
pub(crate) use yaml_loader::YamlLoader;

#[cfg(feature = "decrypt-properties")]
pub use property_decryptor::AesPropertyDecryptor;
