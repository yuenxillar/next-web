//! The resources that config data is loaded from.

use std::fmt;
use std::path::Path;

use next_web_core::io::ResourceLoader;

use crate::context::config::config_data_location::{strip_file_prefix, ConfigDataLocation};

/// The prefix that marks a file system resource key.
const FILE_SYSTEM_KEY_PREFIX: &str = "file:";

/// The prefix that marks a location as optional.
const OPTIONAL_PREFIX: &str = "optional:";

/// The default config file name, without profile and extension.
const CONFIG_NAME: &str = "application";

/// The character that separates the config name from a profile name.
const PROFILE_SEPARATOR: char = '-';

/// The character that separates a file name from its extension.
const EXTENSION_SEPARATOR: char = '.';

/// A resource that config data is loaded from.
///
/// A resource is a concrete file that has been resolved from a
/// [`ConfigDataLocation`], together with the location it originated from and
/// the profile it is specific to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfigDataResource {
    location: ConfigDataLocation,
    path: String,
    file_system: bool,
    profile: Option<String>,
}

impl ConfigDataResource {
    /// Creates a new resource.
    ///
    /// # Arguments
    ///
    /// * `location` - The location the resource was resolved from.
    /// * `path` - The path of the resource, relative to the root of the
    ///   resource loader or to the file system.
    /// * `file_system` - Whether the path is a file system path.
    /// * `profile` - The profile the resource is specific to, when it was
    ///   resolved for a profile.
    pub(crate) fn new(
        location: ConfigDataLocation,
        path: impl Into<String>,
        file_system: bool,
        profile: Option<String>,
    ) -> Self {
        Self {
            location,
            path: path.into(),
            file_system,
            profile,
        }
    }

    /// Returns the location this resource was resolved from.
    pub fn location(&self) -> &ConfigDataLocation {
        &self.location
    }

    /// Returns the path of this resource.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns whether this resource is a file system path.
    pub fn is_file_system(&self) -> bool {
        self.file_system
    }

    /// Returns the profile this resource is specific to, if any.
    pub fn profile(&self) -> Option<&str> {
        self.profile.as_deref()
    }

    /// Returns the key that identifies this resource.
    ///
    /// The key makes sure that a resource is only loaded once, even when it is
    /// resolved by several locations or by several processing phases. It is
    /// also used as the name of the property source that is created for it.
    pub fn key(&self) -> String {
        if self.file_system {
            return format!("{FILE_SYSTEM_KEY_PREFIX}{}", self.path);
        }
        self.path.clone()
    }
}

impl fmt::Display for ConfigDataResource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.key())
    }
}

/// The result of resolving a [`ConfigDataLocation`].
#[derive(Debug, Default)]
pub struct ConfigDataLocationResolution {
    resources: Vec<ConfigDataResource>,
    found: bool,
}

impl ConfigDataLocationResolution {
    /// Creates a new resolution result.
    fn new(resources: Vec<ConfigDataResource>, found: bool) -> Self {
        Self { resources, found }
    }

    /// Returns the resolved resources, in descending precedence order. The
    /// first resource overrides the resources that follow it.
    pub fn resources(&self) -> &[ConfigDataResource] {
        &self.resources
    }

    /// Returns whether the location itself was found.
    ///
    /// A location is found when the resource, directory or wildcard it refers
    /// to exists, even when the directory holds no config file at all.
    pub fn is_found(&self) -> bool {
        self.found
    }
}

/// Resolves config data locations into the resources that are loaded from them.
///
/// A location that refers to a directory is expanded into the default config
/// files that may be present in it, while a location that refers to a file is
/// used as-is.
///
/// The resolver is run again once the active profiles are known: resolving a
/// location for one or more profiles yields the profile specific files of that
/// location as well.
pub struct ConfigDataLocationResolver {
    /// The file extensions that can be loaded, in descending precedence order.
    extensions: Vec<&'static str>,
}

impl ConfigDataLocationResolver {
    /// Creates a new resolver.
    ///
    /// # Arguments
    ///
    /// * `extensions` - The supported file extensions, in descending precedence
    ///   order, without the leading dot.
    pub fn new(extensions: Vec<&'static str>) -> Self {
        Self { extensions }
    }

    /// Resolves the given location into the resources it refers to.
    ///
    /// A single location value may hold several locations separated by
    /// semicolons. The last location that is declared takes precedence, and
    /// resources are returned in descending precedence order, so the first
    /// resource has the highest precedence.
    ///
    /// # Arguments
    ///
    /// * `location` - The location to resolve.
    /// * `profiles` - The active profiles, in ascending precedence order.
    /// * `resource_loader` - The loader used for locations without a `file:`
    ///   prefix.
    pub(crate) fn resolve(
        &self,
        location: &ConfigDataLocation,
        profiles: &[String],
        resource_loader: &dyn ResourceLoader,
    ) -> ConfigDataLocationResolution {
        let mut resources = Vec::new();
        let mut found = false;

        if !location.is_empty() {
            // The last location that is declared wins, so the values are
            // resolved from the last one to the first one.
            for value in location.value().split(';').rev() {
                let value = value.trim();
                if value.is_empty() {
                    continue;
                }
                // An entry may carry its own `optional:` prefix, which is
                // dropped here because the location as a whole decides whether
                // it may be missing.
                let value = value
                    .strip_prefix(OPTIONAL_PREFIX)
                    .map(str::trim)
                    .unwrap_or(value);
                self.resolve_value(
                    location,
                    value,
                    profiles,
                    resource_loader,
                    &mut resources,
                    &mut found,
                );
            }
        }

        ConfigDataLocationResolution::new(resources, found)
    }

    /// Resolves a single location value.
    fn resolve_value(
        &self,
        location: &ConfigDataLocation,
        value: &str,
        profiles: &[String],
        resource_loader: &dyn ResourceLoader,
        resources: &mut Vec<ConfigDataResource>,
        found: &mut bool,
    ) {
        let file_system = value.starts_with("file:");
        let path = resolution_path(value, file_system);

        if value.contains('*') {
            let directories = self.expand_wildcard(path, file_system, resource_loader);
            if !directories.is_empty() {
                *found = true;
            }
            for directory in directories {
                self.resolve_directory(
                    location,
                    &directory,
                    profiles,
                    file_system,
                    resource_loader,
                    resources,
                );
            }
            return;
        }

        if self.is_loadable(path) {
            if self.resource_exists(path, file_system, resource_loader) {
                *found = true;
            }
            self.resolve_file(
                location,
                path,
                profiles,
                file_system,
                resource_loader,
                resources,
            );
            return;
        }

        if self.directory_exists(path, file_system, resource_loader) {
            *found = true;
        }
        self.resolve_directory(
            location,
            path,
            profiles,
            file_system,
            resource_loader,
            resources,
        );
    }

    /// Resolves the default config files of a directory.
    ///
    /// Profile specific files are resolved first, so that a file for a profile
    /// listed later overrides the file of an earlier profile, and both override
    /// the profile independent file of the location.
    fn resolve_directory(
        &self,
        location: &ConfigDataLocation,
        directory: &str,
        profiles: &[String],
        file_system: bool,
        resource_loader: &dyn ResourceLoader,
        resources: &mut Vec<ConfigDataResource>,
    ) {
        for profile in profiles.iter().rev() {
            for extension in &self.extensions {
                let name = format!(
                    "{CONFIG_NAME}{PROFILE_SEPARATOR}{profile}{EXTENSION_SEPARATOR}{extension}"
                );
                self.add_resource(
                    location,
                    directory,
                    &name,
                    file_system,
                    Some(profile.clone()),
                    resource_loader,
                    resources,
                );
            }
        }

        for extension in &self.extensions {
            let name = format!("{CONFIG_NAME}{EXTENSION_SEPARATOR}{extension}");
            self.add_resource(
                location,
                directory,
                &name,
                file_system,
                None,
                resource_loader,
                resources,
            );
        }
    }

    /// Resolves a location that refers to a file.
    ///
    /// Profile specific variants of the default config name are resolved as
    /// well, and take precedence over the file itself.
    fn resolve_file(
        &self,
        location: &ConfigDataLocation,
        path: &str,
        profiles: &[String],
        file_system: bool,
        resource_loader: &dyn ResourceLoader,
        resources: &mut Vec<ConfigDataResource>,
    ) {
        let (directory, name) = split_path(path);
        if let Some((stem, extension)) = split_extension(name) {
            if stem == CONFIG_NAME {
                for profile in profiles.iter().rev() {
                    let variant = format!(
                        "{CONFIG_NAME}{PROFILE_SEPARATOR}{profile}{EXTENSION_SEPARATOR}{extension}"
                    );
                    let variant = join_path(directory, &variant);
                    if self.resource_exists(&variant, file_system, resource_loader) {
                        resources.push(ConfigDataResource::new(
                            location.clone(),
                            variant,
                            file_system,
                            Some(profile.clone()),
                        ));
                    }
                }
            }
        }

        resources.push(ConfigDataResource::new(
            location.clone(),
            path,
            file_system,
            None,
        ));
    }

    /// Adds the resource for the given file of a directory location, unless it
    /// does not exist.
    #[allow(clippy::too_many_arguments)]
    fn add_resource(
        &self,
        location: &ConfigDataLocation,
        directory: &str,
        name: &str,
        file_system: bool,
        profile: Option<String>,
        resource_loader: &dyn ResourceLoader,
        resources: &mut Vec<ConfigDataResource>,
    ) {
        let path = join_path(directory, name);
        if !self.resource_exists(&path, file_system, resource_loader) {
            return;
        }

        resources.push(ConfigDataResource::new(
            location.clone(),
            path,
            file_system,
            profile,
        ));
    }

    /// Returns whether the given path has an extension that can be loaded.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to check.
    pub fn is_loadable(&self, path: &str) -> bool {
        extension_of(path)
            .map(|extension| self.extensions.iter().any(|it| *it == extension))
            .unwrap_or(false)
    }

    /// Returns whether the given resource exists.
    fn resource_exists(
        &self,
        path: &str,
        file_system: bool,
        resource_loader: &dyn ResourceLoader,
    ) -> bool {
        if file_system {
            return Path::new(path).is_file();
        }
        resource_loader.exists(path)
    }

    /// Returns whether the given directory exists.
    fn directory_exists(
        &self,
        directory: &str,
        file_system: bool,
        resource_loader: &dyn ResourceLoader,
    ) -> bool {
        if file_system {
            let directory = if directory.is_empty() { "." } else { directory };
            return Path::new(directory).is_dir();
        }
        !resource_loader.get_directory(directory).is_empty()
    }

    /// Expands a wildcard location into the directories it matches.
    ///
    /// Only the `*/` form is supported: it matches every direct subdirectory of
    /// the directory that precedes the wildcard.
    fn expand_wildcard(
        &self,
        path: &str,
        file_system: bool,
        resource_loader: &dyn ResourceLoader,
    ) -> Vec<String> {
        let Some(wildcard) = path.find('*') else {
            return Vec::new();
        };
        let prefix = path[..wildcard].to_owned();
        let remainder = &path[wildcard + 1..];
        if !(remainder.is_empty() || remainder == "/") {
            return Vec::new();
        }

        let directory = prefix.trim_end_matches('/');
        let mut names = if file_system {
            wildcard_directories(directory)
        } else {
            wildcard_resources(directory, resource_loader)
        };
        names.sort();
        names.dedup();

        names
            .into_iter()
            .map(|name| join_path(directory, &name))
            .collect()
    }
}

/// Returns the normalized path of a location value.
fn resolution_path(value: &str, file_system: bool) -> &str {
    let path = strip_file_prefix(value);
    let path = if file_system {
        path
    } else {
        path.trim_start_matches('/')
    };

    path.trim_end_matches('/')
}

/// Returns the direct subdirectories of the given file system directory.
fn wildcard_directories(directory: &str) -> Vec<String> {
    let directory = if directory.is_empty() { "." } else { directory };
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };

    entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect()
}

/// Returns the direct subdirectories of the given directory of the resources.
fn wildcard_resources(directory: &str, resource_loader: &dyn ResourceLoader) -> Vec<String> {
    let prefix = if directory.is_empty() {
        String::new()
    } else {
        format!("{}/", directory.trim_end_matches('/'))
    };

    let mut names = Vec::new();
    for path in resource_loader.paths() {
        let Some(remainder) = path.strip_prefix(&prefix) else {
            continue;
        };
        let Some((child, rest)) = remainder.split_once('/') else {
            continue;
        };
        if child.is_empty() || rest.is_empty() {
            continue;
        }
        names.push(child.to_owned());
    }
    names
}

/// Joins a directory and a file name.
fn join_path(directory: &str, name: &str) -> String {
    let directory = directory.trim_end_matches('/');
    if directory.is_empty() {
        return name.to_owned();
    }
    format!("{directory}/{name}")
}

/// Splits a path into its directory and its file name.
fn split_path(path: &str) -> (&str, &str) {
    match path.rfind('/') {
        Some(index) => (&path[..index], &path[index + 1..]),
        None => ("", path),
    }
}

/// Splits a file name into its stem and its extension.
fn split_extension(name: &str) -> Option<(&str, &str)> {
    name.rsplit_once(EXTENSION_SEPARATOR)
}

/// Returns the extension of the given path, if it has one.
pub(crate) fn extension_of(path: &str) -> Option<&str> {
    let name = path.rsplit('/').next()?;
    split_extension(name).map(|(_, extension)| extension)
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

    /// Creates a resolver for the extensions of the default loader.
    fn resolver() -> ConfigDataLocationResolver {
        ConfigDataLocationResolver::new(vec!["properties", "xml", "yml", "yaml"])
    }

    /// Returns the given path as a file system location value.
    fn file_location(root: &Path) -> String {
        format!("file:{}/config/", root.display()).replace('\\', "/")
    }

    #[test]
    fn resolves_directory_locations_with_profile_specific_files_first() {
        let root = temp_root("directory");
        write_file(&root, "config/application.properties", "a=1");
        write_file(&root, "config/application-dev.yml", "a=2");
        let resource_loader = resource_loader(&root);

        let resolution = resolver().resolve(
            &ConfigDataLocation::of("optional:/config/"),
            &["dev".to_owned()],
            &resource_loader,
        );

        assert!(resolution.is_found());
        let resolved: Vec<_> = resolution
            .resources()
            .iter()
            .map(|resource| {
                (
                    resource.path().to_owned(),
                    resource.profile().map(ToOwned::to_owned),
                )
            })
            .collect();
        assert_eq!(
            resolved,
            vec![
                (
                    "config/application-dev.yml".to_owned(),
                    Some("dev".to_owned())
                ),
                ("config/application.properties".to_owned(), None),
            ]
        );
    }

    #[test]
    fn resolves_several_locations_within_a_single_value() {
        let root = temp_root("semicolons");
        write_file(&root, "application.properties", "a=1");
        write_file(&root, "config/application.properties", "a=2");
        let resource_loader = resource_loader(&root);

        let resolution = resolver().resolve(
            &ConfigDataLocation::of("optional:/;optional:/config/"),
            &[],
            &resource_loader,
        );

        let paths: Vec<_> = resolution
            .resources()
            .iter()
            .map(|resource| resource.path().to_owned())
            .collect();
        assert_eq!(
            paths,
            vec![
                "config/application.properties".to_owned(),
                "application.properties".to_owned()
            ]
        );
    }

    #[test]
    fn resolves_file_locations_as_is() {
        let root = temp_root("file");
        write_file(&root, "extra/extra.properties", "a=1");
        let resource_loader = resource_loader(&root);

        let resolution = resolver().resolve(
            &ConfigDataLocation::of("/extra/extra.properties"),
            &[],
            &resource_loader,
        );

        assert!(resolution.is_found());
        let paths: Vec<_> = resolution
            .resources()
            .iter()
            .map(|resource| resource.path().to_owned())
            .collect();
        assert_eq!(paths, vec!["extra/extra.properties".to_owned()]);
    }

    #[test]
    fn resolves_profile_specific_variants_of_the_default_config_name() {
        let root = temp_root("default-name");
        write_file(&root, "application.properties", "a=1");
        write_file(&root, "application-dev.properties", "a=2");
        let resource_loader = resource_loader(&root);

        let resolution = resolver().resolve(
            &ConfigDataLocation::of("/application.properties"),
            &["dev".to_owned()],
            &resource_loader,
        );

        let paths: Vec<_> = resolution
            .resources()
            .iter()
            .map(|resource| resource.path().to_owned())
            .collect();
        assert_eq!(
            paths,
            vec![
                "application-dev.properties".to_owned(),
                "application.properties".to_owned()
            ]
        );
    }

    #[test]
    fn resolves_wildcard_directories() {
        let root = temp_root("wildcard");
        write_file(&root, "config/first/application.properties", "a=1");
        write_file(&root, "config/second/application.properties", "a=2");
        let resource_loader = resource_loader(&root);

        let resolution = resolver().resolve(
            &ConfigDataLocation::of("optional:config/*/"),
            &[],
            &resource_loader,
        );

        assert!(resolution.is_found());
        let paths: Vec<_> = resolution
            .resources()
            .iter()
            .map(|resource| resource.path().to_owned())
            .collect();
        assert_eq!(
            paths,
            vec![
                "config/first/application.properties".to_owned(),
                "config/second/application.properties".to_owned()
            ]
        );
    }

    #[test]
    fn resolves_file_system_locations() {
        let root = temp_root("file-system");
        write_file(&root, "config/application.properties", "a=1");
        let resource_loader = resource_loader(&temp_root("file-system-empty"));
        let location = file_location(&root);

        let resolution =
            resolver().resolve(&ConfigDataLocation::of(&location), &[], &resource_loader);

        assert!(resolution.is_found());
        assert_eq!(resolution.resources().len(), 1);
        assert!(resolution.resources()[0].is_file_system());
        assert_eq!(
            resolution.resources()[0].key(),
            format!("{location}application.properties")
        );
    }

    #[test]
    fn reports_missing_locations() {
        let resource_loader = resource_loader(&temp_root("missing"));

        let resolution = resolver().resolve(
            &ConfigDataLocation::of("optional:/missing/"),
            &[],
            &resource_loader,
        );

        assert!(!resolution.is_found());
        assert!(resolution.resources().is_empty());
    }

    #[test]
    fn treats_an_empty_directory_as_found() {
        let root = temp_root("empty-directory");
        fs::create_dir_all(root.join("config")).unwrap();
        let resource_loader = resource_loader(&root);

        let resolution = resolver().resolve(
            &ConfigDataLocation::of(file_location(&root)),
            &[],
            &resource_loader,
        );

        assert!(resolution.is_found());
        assert!(resolution.resources().is_empty());
    }

    #[test]
    fn creates_resource_keys() {
        let location = ConfigDataLocation::of("optional:/config/");
        let resource = ConfigDataResource::new(location, "config/application.yml", false, None);

        assert_eq!(resource.key(), "config/application.yml");
        assert_eq!(resource.to_string(), "config/application.yml");
        assert!(!resource.is_file_system());
        assert_eq!(resource.profile(), None);
    }

    #[test]
    fn detects_loadable_paths() {
        let resolver = resolver();

        assert!(resolver.is_loadable("config/application.yml"));
        assert!(resolver.is_loadable("application.properties"));
        assert!(!resolver.is_loadable("config/application.json"));
        assert!(!resolver.is_loadable("config/"));
    }
}
