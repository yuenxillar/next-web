use std::io;

use next_web_core::{env::PropertySource, io::Resource, util::indexmap::IndexMap};

/// Strategy trait used to load a [`PropertySource`].
///
/// Implementations are typically discovered dynamically (for example through a
/// service registry) rather than instantiated directly. Each loader declares
/// the file extensions it understands and knows how to turn a resource into
/// one or more property sources.
pub trait PropertySourceLoader {
    /// Return the file extensions that the loader supports, excluding the
    /// leading dot.
    ///
    /// For example, a loader for YAML files would return `["yml", "yaml"]`.
    fn file_extensions(&self) -> &[&'static str];

    /// Load the resource into one or more property sources.
    ///
    /// Implementations may either return a list containing a single source, or
    /// — in the case of a multi-document format such as YAML — one source per
    /// document in the resource.
    ///
    /// `name` is the root name of the property source. If multiple documents
    /// are loaded, an additional suffix should be added to the name for each
    /// source loaded.
    ///
    /// `resource` is the resource to load.
    ///
    /// Returns a list of property sources. Returns an [`io::Error`] if the
    /// source cannot be loaded.
    fn load(
        &self,
        name: &str,
        resource: &dyn Resource,
    ) -> io::Result<Vec<Box<dyn PropertySource<IndexMap<String, String>>>>>;
}
