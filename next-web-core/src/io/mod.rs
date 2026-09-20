mod bytes_resource;
mod config_only_policy;
mod default_resource_loader;
mod file_resource;
mod resource;
mod resource_access_policy;
mod resource_loader;

pub use bytes_resource::BytesResource;
pub use config_only_policy::ConfigOnlyPolicy;
pub use default_resource_loader::{DefaultResourceLoader, ResourceCacheConfig};

// mod embed_resource_loader;
// pub use embed_resource_loader::EmbedResourceLoader;
pub use file_resource::{FileResource, Metadata};
pub use resource::Resource;
pub use resource_access_policy::ResourceAccessPolicy;
pub use resource_loader::ResourceLoader;
