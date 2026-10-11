//! The resource loader the web layer of the process reads from.
//!
//! [`DefaultResourceLoader::shared`] is the loader the framework reads its own
//! configuration with, and it is created while the configuration data is read,
//! before the configuration it holds can be known. The loader of this module is
//! the one the web layer reads the views and the resources of a dispatch from,
//! and an application installs it while it starts, once the resources of the
//! application are configured:
//!
//! ```ignore
//! let loader: Arc<dyn ResourceLoader> = /* built from the environment */;
//! let _ = install_web_resource_loader(loader);
//! ```
//!
//! Nothing installed means the shared loader is used, so an application that
//! configures nothing reads the same resources as the rest of the framework.

use std::sync::{Arc, OnceLock};

use crate::io::{DefaultResourceLoader, ResourceLoader};

/// The loader the web layer reads from, once an application installed one.
static WEB_RESOURCE_LOADER: OnceLock<Arc<dyn ResourceLoader>> = OnceLock::new();

/// Installs the resource loader the web layer reads from.
///
/// The first installation wins, which is the loader of the first application of
/// the process; a later application keeps its own loader in its context, but the
/// views and the dispatches of the process read the loader installed here. The
/// call is not an error when a loader is installed already: it answers the
/// installed one, so a caller can tell that its loader was not the one used.
///
/// # Examples
///
/// ```ignore
/// if let Err(installed) = install_web_resource_loader(loader) {
///     // another loader was installed first, which is the one in use
/// }
/// ```
pub fn install_web_resource_loader(
    loader: Arc<dyn ResourceLoader>,
) -> Result<(), Arc<dyn ResourceLoader>> {
    WEB_RESOURCE_LOADER.set(loader)
}

/// Returns the resource loader the web layer reads from.
///
/// The loader an application installed wins; without one the shared loader of
/// [`DefaultResourceLoader`] is returned, so a request answered before an
/// application installed a loader still finds the resources of the `resources`
/// directory.
pub fn web_resource_loader() -> Arc<dyn ResourceLoader> {
    match WEB_RESOURCE_LOADER.get() {
        Some(loader) => Arc::clone(loader),
        None => DefaultResourceLoader::shared_arc(),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::Arc};

    use super::{install_web_resource_loader, web_resource_loader};
    use crate::io::{DefaultResourceLoader, ResourceLoader};

    #[test]
    fn the_web_layer_reads_the_installed_loader() {
        // Nothing is installed yet, so the web layer reads the shared loader of
        // the framework.
        let shared: Arc<dyn ResourceLoader> = DefaultResourceLoader::shared_arc();
        assert!(Arc::ptr_eq(&web_resource_loader(), &shared));

        let root = std::env::temp_dir().join(format!(
            "next-web-web-resource-loader-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("index.html"), "<html/>").unwrap();

        let mut loader = DefaultResourceLoader::new(&root, None);
        loader.load().unwrap();
        let installed: Arc<dyn ResourceLoader> = Arc::new(loader);

        // The first installation is the loader the web layer reads from.
        assert!(install_web_resource_loader(Arc::clone(&installed)).is_ok());
        assert!(Arc::ptr_eq(&web_resource_loader(), &installed));
        assert!(web_resource_loader().exists("index.html"));

        // A second installation is refused, and the first loader stays in use.
        assert!(install_web_resource_loader(DefaultResourceLoader::shared_arc()).is_err());
        assert!(Arc::ptr_eq(&web_resource_loader(), &installed));

        let _ = fs::remove_dir_all(&root);
    }
}
