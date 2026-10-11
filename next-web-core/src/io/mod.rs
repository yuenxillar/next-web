pub mod support;

mod allow_list;
mod bytes_resource;
mod config_only_policy;
mod default_resource_loader;
mod file_resource;
mod resource;
mod resource_access_policy;
mod resource_loader;
mod web_resource_loader;
mod web_resource_policy;

pub(crate) use allow_list::allows;
pub use bytes_resource::BytesResource;
pub use config_only_policy::ConfigOnlyPolicy;
pub use default_resource_loader::{DefaultResourceLoader, ResourceCacheConfig};

pub use file_resource::{FileResource, Metadata};
pub use resource::Resource;
pub use resource_access_policy::ResourceAccessPolicy;
pub use resource_loader::ResourceLoader;
pub use web_resource_loader::{install_web_resource_loader, web_resource_loader};
pub use web_resource_policy::WebResourcePolicy;
