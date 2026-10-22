//! Building blocks shared by the builders and configurers.
/// The base of every builder, which keeps the built object.
pub mod base_builder;
/// The base of every configured builder, which keeps the registered configurers.
pub mod base_configured_builder;
/// The builder contract.
pub mod builder;
/// The configurer contract.
pub mod configurer;
/// Adapter which lets a configurer implement only the callbacks it needs.
pub mod configurer_adapter;
/// Post processor applied to the objects a builder creates.
pub mod object_post_processor;

pub use base_builder::BaseBuilder;
pub use base_configured_builder::BaseConfiguredBuilder;
pub use builder::Builder;
pub use configurer::Configurer;
pub use configurer_adapter::ConfigurerAdapter;
pub use object_post_processor::ObjectPostProcessor;
