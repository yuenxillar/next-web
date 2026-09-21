//! Loads config data from the resources that have been resolved from locations.

use std::io;
use std::sync::Arc;

use next_web_core::{
    env::{BoxedPropertySource, PropertySource, PropertySourceValue},
    io::{FileResource, Resource, ResourceLoader},
};

use crate::context::config::config_data_location::ConfigDataLocation;
use crate::context::config::config_data_resource::{extension_of, ConfigDataResource};
use crate::env::{PropertiesPropertySourceLoader, PropertySourceLoader, YamlPropertySourceLoader};

/// The property that declares the profile expression a document is active for.
pub(crate) const ACTIVATE_PROFILE_PROPERTY: &str = "next.config.activate.on-profile";

/// The property used to declare additional locations to import.
pub(crate) const IMPORT_PROPERTY: &str = "next.config.import";

/// Config data loaded from a single document of a resource.
///
/// A resource that holds several documents, such as a multi document YAML file,
/// produces one instance per document, so that every document can carry its own
/// activation properties.
pub struct ConfigData {
    property_source: BoxedPropertySource,
    activate_profile: Option<String>,
}

impl ConfigData {
    /// Creates config data for the given property source.
    ///
    /// # Arguments
    ///
    /// * `property_source` - The property source of a single document.
    pub(crate) fn new(property_source: BoxedPropertySource) -> Self {
        let activate_profile = property_source
            .property(ACTIVATE_PROFILE_PROPERTY)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());

        Self {
            property_source,
            activate_profile,
        }
    }

    /// Returns the property source of this config data.
    pub fn property_source(&self) -> &dyn PropertySource<PropertySourceValue> {
        self.property_source.as_ref()
    }

    /// Returns the profile expression this document is active for, if any.
    ///
    /// The expression is taken from the `next.config.activate.on-profile`
    /// property of the document.
    pub fn activate_profile(&self) -> Option<&str> {
        self.activate_profile.as_deref()
    }

    /// Returns the locations imported by this document.
    ///
    /// The locations are taken from the `next.config.import` property of the
    /// document.
    pub fn imports(&self) -> Vec<ConfigDataLocation> {
        self.property_source
            .property(IMPORT_PROPERTY)
            .map(|value| config_data_locations(&value))
            .unwrap_or_default()
    }

    /// Returns the property source of this config data.
    pub fn into_property_source(self) -> BoxedPropertySource {
        self.property_source
    }
}

/// Parses a comma separated list of config data locations.
///
/// # Arguments
///
/// * `value` - The value of a config data location property.
pub(crate) fn config_data_locations(value: &str) -> Vec<ConfigDataLocation> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ConfigDataLocation::of)
        .collect()
}

/// Loads config data from the resolved resources.
///
/// Resources are dispatched to the registered property source loaders by their
/// file extension. The order of the loaders matters: the extensions of the
/// loader that is registered first have the highest precedence, so a
/// `.properties` file overrides an equivalent YAML file in the same location.
#[derive(Clone)]
pub struct ConfigDataLoader {
    loaders: Arc<[Box<dyn PropertySourceLoader>]>,
}

impl ConfigDataLoader {
    /// Creates a loader using the given property source loaders.
    ///
    /// # Arguments
    ///
    /// * `loaders` - The property source loaders, in descending precedence
    ///   order.
    pub fn new(loaders: Vec<Box<dyn PropertySourceLoader>>) -> Self {
        Self {
            loaders: Arc::from(loaders),
        }
    }

    /// Returns the supported file extensions, in descending precedence order.
    pub fn extensions(&self) -> Vec<&'static str> {
        self.loaders
            .iter()
            .flat_map(|loader| loader.file_extensions().iter().copied())
            .collect()
    }

    /// Loads the config data of the given resource.
    ///
    /// # Arguments
    ///
    /// * `resource` - The resource to load.
    /// * `resource_loader` - The loader used for locations without a `file:`
    ///   prefix.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] when the resource cannot be read.
    pub(crate) fn load(
        &self,
        resource: &ConfigDataResource,
        resource_loader: &dyn ResourceLoader,
    ) -> io::Result<Vec<ConfigData>> {
        let name = resource.key();

        if resource.is_file_system() {
            let file = FileResource::new(resource.path())?;
            return self.load_source(&name, &file);
        }

        let source = resource_loader.get_resource(resource.path())?;
        self.load_source(&name, source)
    }

    /// Loads a single resource into its config data.
    fn load_source(&self, name: &str, resource: &dyn Resource) -> io::Result<Vec<ConfigData>> {
        let Some(loader) = self.loader_for_path(name) else {
            return Ok(Vec::new());
        };

        let property_sources = loader.load(name, resource)?;
        Ok(property_sources.into_iter().map(ConfigData::new).collect())
    }

    /// Returns the property source loader that supports the given path.
    fn loader_for_path(&self, path: &str) -> Option<&dyn PropertySourceLoader> {
        let extension = extension_of(path)?;
        self.loaders
            .iter()
            .find(|loader| loader.file_extensions().iter().any(|it| *it == extension))
            .map(|loader| loader.as_ref())
    }
}

impl Default for ConfigDataLoader {
    /// Creates a loader holding the property source loaders of the framework.
    fn default() -> Self {
        Self::new(vec![
            Box::new(PropertiesPropertySourceLoader),
            Box::new(YamlPropertySourceLoader),
        ])
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use next_web_core::io::DefaultResourceLoader;

    use super::*;

    /// Creates a unique temporary directory for the duration of a test.
    fn temp_root(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "next-web-config-data-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    /// Writes the given content to `root/relative`, creating parents.
    fn write_file(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    /// Creates a loaded resource loader for the given root.
    fn resource_loader(root: &Path) -> DefaultResourceLoader {
        let mut loader = DefaultResourceLoader::new(root, None);
        loader.load().unwrap();
        loader
    }

    /// Returns the resource for the given path of the resources.
    fn resource(path: &str) -> ConfigDataResource {
        ConfigDataResource::new(ConfigDataLocation::of("/"), path, false, None)
    }

    #[test]
    fn lists_the_supported_extensions_in_descending_precedence() {
        assert_eq!(
            ConfigDataLoader::default().extensions(),
            vec!["properties", "xml", "yml", "yaml"]
        );
    }

    #[test]
    fn loads_properties_documents() {
        let root = temp_root("load-properties");
        write_file(
            &root,
            "application.properties",
            "next.application.name=demo",
        );
        let resource_loader = resource_loader(&root);

        let config_data = ConfigDataLoader::default()
            .load(&resource("application.properties"), &resource_loader)
            .unwrap();

        assert_eq!(config_data.len(), 1);
        assert_eq!(
            config_data[0]
                .property_source()
                .property("next.application.name"),
            Some("demo".to_owned())
        );
        assert_eq!(config_data[0].activate_profile(), None);
        assert!(config_data[0].imports().is_empty());
    }

    #[test]
    fn loads_yaml_documents() {
        let root = temp_root("load-yaml");
        write_file(
            &root,
            "application.yml",
            "next:\n  application:\n    name: demo\n",
        );
        let resource_loader = resource_loader(&root);

        let config_data = ConfigDataLoader::default()
            .load(&resource("application.yml"), &resource_loader)
            .unwrap();

        assert_eq!(config_data.len(), 1);
        assert_eq!(
            config_data[0]
                .property_source()
                .property("next.application.name"),
            Some("demo".to_owned())
        );
    }

    #[test]
    fn reads_activation_profiles_and_imports() {
        let root = temp_root("activation");
        write_file(
            &root,
            "application-dev.properties",
            "next.config.activate.on-profile=dev\nnext.config.import=/extra.properties,optional:file:./extra.yml\n",
        );
        let resource_loader = resource_loader(&root);

        let config_data = ConfigDataLoader::default()
            .load(&resource("application-dev.properties"), &resource_loader)
            .unwrap();

        assert_eq!(config_data[0].activate_profile(), Some("dev"));
        assert_eq!(
            config_data[0].imports(),
            vec![
                ConfigDataLocation::of("/extra.properties"),
                ConfigDataLocation::of("optional:file:./extra.yml"),
            ]
        );
    }

    #[test]
    fn ignores_resources_without_a_supported_extension() {
        let root = temp_root("unsupported");
        write_file(&root, "application.json", "{}");
        let resource_loader = resource_loader(&root);

        assert!(ConfigDataLoader::default()
            .load(&resource("application.json"), &resource_loader)
            .unwrap()
            .is_empty());
    }
}
