//! Loads and applies config data to an environment.

use std::collections::BTreeSet;
use std::fmt;

use next_web_core::{
    env::{ConfigurableEnvironment, MutablePropertySources},
    io::ResourceLoader,
};
use tracing::trace;

use crate::context::config::config_data_activation_context::ConfigDataActivationContext;
use crate::context::config::config_data_environment_contributor::ConfigDataEnvironmentContributor;
use crate::context::config::config_data_environment_contributors::ConfigDataEnvironmentContributors;
use crate::context::config::config_data_environment_update_listener::ConfigDataEnvironmentUpdateListener;
use crate::context::config::config_data_importer::ConfigDataImporter;
use crate::context::config::config_data_loader::{
    config_data_locations, ConfigDataLoader, IMPORT_PROPERTY,
};
use crate::context::config::config_data_location::ConfigDataLocation;
use crate::context::config::config_data_location_not_found_error::ConfigDataLocationNotFoundError;
use crate::context::config::config_data_not_found_action::ConfigDataNotFoundAction;
use crate::context::config::config_data_profiles::{
    profiles_from_value, ConfigDataProfiles, ACTIVE_PROFILE_PROPERTY, DEFAULT_PROFILE_PROPERTY,
    INCLUDE_PROFILES,
};
use crate::context::config::invalid_config_data_property_error::InvalidConfigDataPropertyError;
use crate::env::DefaultPropertiesPropertySource;

/// Property used to override the imported locations.
pub(crate) const LOCATION_PROPERTY: &str = "next.config.location";

/// Property used to provide additional locations to import.
pub(crate) const ADDITIONAL_LOCATION_PROPERTY: &str = "next.config.additional-location";

/// Property used to determine what action to take when a location that must
/// exist cannot be found.
pub(crate) const ON_NOT_FOUND_PROPERTY: &str = "next.config.on-not-found";

/// Returns the locations that are searched when `next.config.location` is not
/// set.
///
/// The locations are registered one by one in reverse order, so that the last
/// location that is listed takes precedence: the file system locations are
/// therefore searched before the locations of the resources.
fn default_search_locations() -> Vec<ConfigDataLocation> {
    vec![
        ConfigDataLocation::of("optional:/;optional:/config/"),
        ConfigDataLocation::of(
            "optional:file:./;optional:file:./config/;optional:file:./config/*/",
        ),
    ]
}

/// Error raised while config data is processed.
#[derive(Debug)]
pub enum ConfigDataEnvironmentError {
    /// A location that must exist could not be found.
    LocationNotFound(ConfigDataLocationNotFoundError),
    /// An invalid property was found in the config data.
    InvalidProperty(InvalidConfigDataPropertyError),
}

impl fmt::Display for ConfigDataEnvironmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocationNotFound(error) => error.fmt(f),
            Self::InvalidProperty(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ConfigDataEnvironmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::LocationNotFound(error) => Some(error),
            Self::InvalidProperty(error) => Some(error),
        }
    }
}

impl From<ConfigDataLocationNotFoundError> for ConfigDataEnvironmentError {
    fn from(error: ConfigDataLocationNotFoundError) -> Self {
        Self::LocationNotFound(error)
    }
}

impl From<InvalidConfigDataPropertyError> for ConfigDataEnvironmentError {
    fn from(error: InvalidConfigDataPropertyError) -> Self {
        Self::InvalidProperty(error)
    }
}

/// A wrapper around a [`ConfigurableEnvironment`] that imports and applies
/// config data.
///
/// The initial locations are taken from the `next.config.import`,
/// `next.config.additional-location` and `next.config.location` properties of
/// the environment, and default to the default search locations when
/// `next.config.location` is not set.
///
/// The locations are processed in phases:
///
/// 1. Every location is resolved and loaded without an activation context, so
///    that the properties that declare the active profiles are known.
/// 2. The profiles are deduced from the properties that were loaded.
/// 3. Every location is resolved and loaded again with the profiles in place,
///    which adds the profile specific documents of the locations.
/// 4. The resulting property sources are applied to the environment, and the
///    active and default profiles of the environment are updated.
///
/// A resource that was already loaded is not loaded a second time, and a
/// location that cannot be found either fails or is ignored, depending on the
/// `next.config.on-not-found` property.
pub(crate) struct ConfigDataEnvironment<'a> {
    not_found_action: ConfigDataNotFoundAction,
    environment: &'a mut dyn ConfigurableEnvironment,
    resource_loader: &'a dyn ResourceLoader,
    loader: ConfigDataLoader,
    additional_profiles: Vec<String>,
    update_listener: &'a dyn ConfigDataEnvironmentUpdateListener,
    contributors: ConfigDataEnvironmentContributors,
}

impl<'a> ConfigDataEnvironment<'a> {
    /// Creates a new config data environment for the given environment.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment to load config data for.
    /// * `resource_loader` - The loader used for locations without a `file:`
    ///   prefix.
    /// * `loader` - The loader used for the resources of a location.
    /// * `additional_profiles` - Profiles to activate on top of the profiles
    ///   that are declared by the environment and the config data.
    /// * `update_listener` - Listener notified of the changes that are made.
    pub(crate) fn new(
        environment: &'a mut dyn ConfigurableEnvironment,
        resource_loader: &'a dyn ResourceLoader,
        loader: ConfigDataLoader,
        additional_profiles: Vec<String>,
        update_listener: &'a dyn ConfigDataEnvironmentUpdateListener,
    ) -> Self {
        let not_found_action = environment_property(environment, ON_NOT_FOUND_PROPERTY)
            .and_then(|value| ConfigDataNotFoundAction::of(&value))
            .unwrap_or_default();
        let contributors = Self::create_contributors(environment);

        Self {
            not_found_action,
            environment,
            resource_loader,
            loader,
            additional_profiles,
            update_listener,
            contributors,
        }
    }

    /// Processes every location and applies the result to the environment.
    ///
    /// # Errors
    ///
    /// Returns an error when a location that must exist cannot be found, or
    /// when the config data holds a property that is not allowed.
    pub(crate) fn process_and_apply(&mut self) -> Result<(), ConfigDataEnvironmentError> {
        let mut importer = ConfigDataImporter::new(
            self.resource_loader,
            self.loader.clone(),
            self.not_found_action,
        );

        let activation_context = self.process_initial(&mut importer)?;
        let activation_context = self.with_profiles(&activation_context);
        self.process_with_profiles(&mut importer, &activation_context)?;

        self.apply_to_environment(&activation_context, &importer)
    }

    /// Processes the pending imports without an activation context.
    fn process_initial(
        &mut self,
        importer: &mut ConfigDataImporter<'_>,
    ) -> Result<ConfigDataActivationContext, ConfigDataLocationNotFoundError> {
        trace!("Processing config data locations without an activation context");
        let activation_context = ConfigDataActivationContext::new(None);

        self.contributors
            .expand_imports(importer, &activation_context)?;

        Ok(activation_context)
    }

    /// Deduces the profiles from the loaded documents and returns an activation
    /// context holding them.
    fn with_profiles(
        &mut self,
        activation_context: &ConfigDataActivationContext,
    ) -> ConfigDataActivationContext {
        trace!("Deducing profiles from the config data");
        activation_context.with_profiles(self.create_profiles())
    }

    /// Processes the pending imports with the profiles in place.
    ///
    /// Resources that were already loaded are not loaded again, so this phase
    /// only adds the documents that are specific to the active profiles.
    fn process_with_profiles(
        &mut self,
        importer: &mut ConfigDataImporter<'_>,
        activation_context: &ConfigDataActivationContext,
    ) -> Result<(), ConfigDataLocationNotFoundError> {
        trace!("Processing config data locations with an activation context");

        self.contributors
            .expand_imports(importer, activation_context)
    }

    /// Creates the profiles from the environment and from the loaded documents.
    fn create_profiles(&mut self) -> ConfigDataProfiles {
        let active = match self.profile_property(ACTIVE_PROFILE_PROPERTY) {
            Some(value) => profiles_from_value(&value),
            None => self.environment.active_profiles().to_vec(),
        };
        let default = match self.profile_property(DEFAULT_PROFILE_PROPERTY) {
            Some(value) => profiles_from_value(&value),
            None => self.environment.default_profiles().to_vec(),
        };

        let mut active = active;
        active.extend(self.additional_profiles.iter().cloned());
        for value in self.profile_property_values(INCLUDE_PROFILES) {
            active.extend(profiles_from_value(&value));
        }

        ConfigDataProfiles::new(active, default)
    }

    /// Returns the value of the given profile property, taken from the
    /// environment first and from the loaded documents second.
    fn profile_property(&mut self, name: &str) -> Option<String> {
        environment_property(self.environment, name)
            .or_else(|| self.contributors.get_profile_property(name))
    }

    /// Returns the values of the given profile property, taken from the
    /// environment and from the loaded documents.
    fn profile_property_values(&mut self, name: &str) -> Vec<String> {
        let mut values: Vec<String> = self
            .environment
            .property_sources()
            .iter()
            .filter_map(|source| source.property(name))
            .collect();
        values.extend(self.contributors.get_profile_property_values(name));
        values
    }

    /// Applies the processed contributors to the environment.
    fn apply_to_environment(
        &mut self,
        activation_context: &ConfigDataActivationContext,
        importer: &ConfigDataImporter<'_>,
    ) -> Result<(), ConfigDataEnvironmentError> {
        self.check_for_invalid_properties()?;

        let contributors = std::mem::take(&mut self.contributors);
        self.check_mandatory_locations(
            &contributors,
            activation_context,
            importer.loaded_locations(),
            importer.optional_locations(),
        )?;

        let update_listener = self.update_listener;
        let property_sources = self.environment.property_sources();
        for contributor in contributors.into_flattened() {
            apply_contributor(
                contributor,
                activation_context,
                property_sources,
                update_listener,
            );
        }

        DefaultPropertiesPropertySource::move_to_end(self.environment);

        let profiles = activation_context
            .get_profiles()
            .expect("'profiles' must not be null");
        trace!(
            "Setting default profiles: {:?}",
            profiles.default_profiles()
        );
        let default_profiles = as_refs(profiles.default_profiles());
        self.environment.set_default_profiles(&default_profiles);
        trace!("Setting active profiles: {:?}", profiles.active());
        let active_profiles = as_refs(profiles.active());
        self.environment.set_active_profiles(&active_profiles);
        self.update_listener.on_set_profiles(profiles);

        Ok(())
    }

    /// Returns an error when the config data holds a property that is not
    /// allowed.
    fn check_for_invalid_properties(&self) -> Result<(), InvalidConfigDataPropertyError> {
        for contributor in self.contributors.flattened() {
            InvalidConfigDataPropertyError::check(contributor)?;
        }
        Ok(())
    }

    /// Reports the locations that must exist but could not be found.
    fn check_mandatory_locations(
        &self,
        contributors: &ConfigDataEnvironmentContributors,
        activation_context: &ConfigDataActivationContext,
        loaded_locations: &BTreeSet<ConfigDataLocation>,
        optional_locations: &BTreeSet<ConfigDataLocation>,
    ) -> Result<(), ConfigDataLocationNotFoundError> {
        let mut mandatory_locations: Vec<ConfigDataLocation> = Vec::new();
        for contributor in contributors.flattened() {
            if !contributor.is_active(activation_context) {
                continue;
            }
            for location in contributor.mandatory_imports() {
                if !mandatory_locations.contains(&location) {
                    mandatory_locations.push(location);
                }
            }
        }

        for contributor in contributors.flattened() {
            if let Some(location) = contributor.location() {
                mandatory_locations.retain(|mandatory| mandatory != location);
            }
        }

        mandatory_locations.retain(|location| {
            !loaded_locations.contains(location) && !optional_locations.contains(location)
        });

        for location in mandatory_locations {
            self.not_found_action
                .handle(ConfigDataLocationNotFoundError::new(location))?;
        }

        Ok(())
    }

    /// Creates the initial contributors of the environment.
    fn create_contributors(
        environment: &mut dyn ConfigurableEnvironment,
    ) -> ConfigDataEnvironmentContributors {
        trace!("Building config data environment contributors");

        let imports = environment_locations(environment, IMPORT_PROPERTY, Vec::new());
        let additional =
            environment_locations(environment, ADDITIONAL_LOCATION_PROPERTY, Vec::new());
        let locations =
            environment_locations(environment, LOCATION_PROPERTY, default_search_locations());

        let mut contributors = Vec::new();
        add_import_contributors(&mut contributors, imports, false);
        add_import_contributors(&mut contributors, additional, true);
        add_import_contributors(&mut contributors, locations, true);

        ConfigDataEnvironmentContributors::new(contributors)
    }
}

/// Returns the value of the given property, taken from the first property
/// source of the environment that contains it.
fn environment_property(
    environment: &mut dyn ConfigurableEnvironment,
    name: &str,
) -> Option<String> {
    environment
        .property_sources()
        .iter()
        .find_map(|source| source.property(name))
}

/// Returns the locations declared by the given property, or the given default
/// when the property is absent.
fn environment_locations(
    environment: &mut dyn ConfigurableEnvironment,
    name: &str,
    default: Vec<ConfigDataLocation>,
) -> Vec<ConfigDataLocation> {
    environment_property(environment, name)
        .map(|value| config_data_locations(&value))
        .unwrap_or(default)
}

/// Adds a contributor for the given locations.
///
/// When the locations are registered individually, they are added in reverse
/// order, so that the last location that is declared takes precedence.
fn add_import_contributors(
    contributors: &mut Vec<ConfigDataEnvironmentContributor>,
    locations: Vec<ConfigDataLocation>,
    register_individually: bool,
) {
    if locations.is_empty() {
        return;
    }

    if register_individually {
        for location in locations.into_iter().rev() {
            contributors.push(ConfigDataEnvironmentContributor::initial_imports(vec![
                location,
            ]));
        }
    } else {
        contributors.push(ConfigDataEnvironmentContributor::initial_imports(locations));
    }
}

/// Adds the property source of the given contributor to the environment.
fn apply_contributor(
    contributor: ConfigDataEnvironmentContributor,
    activation_context: &ConfigDataActivationContext,
    property_sources: &mut MutablePropertySources,
    update_listener: &dyn ConfigDataEnvironmentUpdateListener,
) {
    if !contributor.is_active(activation_context) {
        if let Some(property_source) = contributor.property_source() {
            trace!(
                "Skipping inactive property source '{}'",
                property_source.name()
            );
        }
        return;
    }

    let location = contributor.location().cloned();
    let Some(property_source) = contributor.into_property_source() else {
        return;
    };

    let name = property_source.name().to_owned();
    trace!("Adding config data property source '{name}'");
    property_sources.add_last(property_source);
    update_listener.on_property_source_added(&name, location.as_ref());
}

/// Returns the given profiles as string references.
fn as_refs(profiles: &[String]) -> Vec<&str> {
    profiles.iter().map(String::as_str).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    use next_web_core::env::{
        ConfigurablePropertyResolver, Environment, MutablePropertySources, PropertyResolver,
    };
    use next_web_core::io::DefaultResourceLoader;
    use next_web_core::util::indexmap::IndexMap;

    use crate::context::config::NoOpConfigDataEnvironmentUpdateListener;
    use crate::env::MapPropertySource;

    use super::*;

    /// A listener that records the updates it is told about.
    #[derive(Default)]
    struct RecordingUpdateListener {
        added: Mutex<Vec<String>>,
        profiles: Mutex<Vec<String>>,
    }

    impl ConfigDataEnvironmentUpdateListener for RecordingUpdateListener {
        fn on_property_source_added(
            &self,
            property_source: &str,
            _location: Option<&ConfigDataLocation>,
        ) {
            self.added.lock().unwrap().push(property_source.to_owned());
        }

        fn on_set_profiles(&self, profiles: &ConfigDataProfiles) {
            *self.profiles.lock().unwrap() = profiles.active().to_vec();
        }
    }

    /// A minimal environment, holding its property sources in memory.
    #[derive(Default)]
    struct MockEnvironment {
        property_sources: MutablePropertySources,
        active_profiles: Vec<String>,
        default_profiles: Vec<String>,
    }

    impl MockEnvironment {
        /// Adds a map backed property source with the given properties.
        fn with_property_source(mut self, name: &str, properties: &[(&str, &str)]) -> Self {
            let properties: IndexMap<String, String> = properties
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect();
            self.property_sources
                .add_last(Box::new(MapPropertySource::new(
                    name.to_owned(),
                    properties,
                )));
            self
        }

        /// Returns the names of the property sources of this environment.
        fn source_names(&self) -> Vec<String> {
            self.property_sources
                .iter()
                .map(|source| source.name().to_owned())
                .collect()
        }

        /// Returns the value of the given property.
        fn property(&self, name: &str) -> Option<String> {
            self.property_sources
                .iter()
                .find_map(|source| source.property(name))
        }
    }

    impl PropertyResolver for MockEnvironment {
        fn contains_property(&self, key: &str) -> bool {
            self.property(key).is_some()
        }

        fn get_property(&self, key: &str) -> Option<String> {
            self.property(key)
        }

        fn get_property_or_default(&self, _key: &str, _default_value: &str) -> String {
            unimplemented!("not needed to process config data")
        }

        fn get_required_property(
            &self,
            _key: &str,
        ) -> Result<String, next_web_core::error::IllegalError> {
            unimplemented!("not needed to process config data")
        }

        fn resolve_placeholders(&self, _text: &str) -> String {
            unimplemented!("not needed to process config data")
        }

        fn resolve_required_placeholders(
            &self,
            _text: &str,
        ) -> Result<String, next_web_core::error::IllegalError> {
            unimplemented!("not needed to process config data")
        }
    }

    impl ConfigurablePropertyResolver for MockEnvironment {}

    impl Environment for MockEnvironment {
        fn active_profiles(&self) -> &[String] {
            &self.active_profiles
        }

        fn default_profiles(&self) -> &[String] {
            &self.default_profiles
        }

        fn accepts_profiles(&self, profiles: &dyn next_web_core::env::Profiles) -> bool {
            profiles.matches(&|profile: &str| {
                self.active_profiles.iter().any(|active| active == profile)
            })
        }
    }

    impl ConfigurableEnvironment for MockEnvironment {
        fn set_active_profiles(&mut self, profiles: &[&str]) {
            self.active_profiles = profiles
                .iter()
                .map(|profile| (*profile).to_owned())
                .collect();
        }

        fn add_active_profile(&mut self, profile: &str) {
            self.active_profiles.push(profile.to_owned());
        }

        fn set_default_profiles(&mut self, profiles: &[&str]) {
            self.default_profiles = profiles
                .iter()
                .map(|profile| (*profile).to_owned())
                .collect();
        }

        fn property_sources(&mut self) -> &mut MutablePropertySources {
            &mut self.property_sources
        }

        fn property_sources_ref(&self) -> &MutablePropertySources {
            &self.property_sources
        }

        fn system_properties(&self) -> HashMap<String, String> {
            HashMap::new()
        }

        fn system_environment(&self) -> HashMap<String, String> {
            HashMap::new()
        }

        fn merge(&mut self, _parent: &dyn ConfigurableEnvironment) {}
    }

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

    /// Processes config data for the given environment and resource loader.
    fn process(
        environment: &mut MockEnvironment,
        resource_loader: &DefaultResourceLoader,
    ) -> Result<(), ConfigDataEnvironmentError> {
        let mut config_data = ConfigDataEnvironment::new(
            environment,
            resource_loader,
            ConfigDataLoader::default(),
            Vec::new(),
            &NoOpConfigDataEnvironmentUpdateListener,
        );

        config_data.process_and_apply()
    }

    #[test]
    fn applies_the_default_search_locations() {
        let root = temp_root("default-locations");
        write_file(
            &root,
            "application.properties",
            "next.application.name=root",
        );
        write_file(
            &root,
            "config/application.properties",
            "next.application.name=config",
        );
        let resource_loader = resource_loader(&root);
        let mut environment = MockEnvironment::default();

        process(&mut environment, &resource_loader).unwrap();

        assert_eq!(
            environment.source_names(),
            vec![
                "config/application.properties".to_owned(),
                "application.properties".to_owned(),
            ]
        );
        assert_eq!(
            environment.property("next.application.name"),
            Some("config".to_owned())
        );
    }

    #[test]
    fn applies_profile_specific_documents_and_updates_the_profiles() {
        let root = temp_root("profiles");
        write_file(
            &root,
            "application.properties",
            "next.application.name=base\nnext.profiles.active=dev\n",
        );
        write_file(
            &root,
            "application-dev.properties",
            "next.application.name=dev\nnext.application.port=8080\n",
        );
        let resource_loader = resource_loader(&root);
        let mut environment = MockEnvironment::default();

        process(&mut environment, &resource_loader).unwrap();

        assert_eq!(
            environment.source_names(),
            vec![
                "application-dev.properties".to_owned(),
                "application.properties".to_owned(),
            ]
        );
        assert_eq!(
            environment.property("next.application.name"),
            Some("dev".to_owned())
        );
        assert_eq!(environment.active_profiles, vec!["dev".to_owned()]);
    }

    #[test]
    fn imported_documents_take_precedence_over_the_importing_document() {
        let root = temp_root("imports");
        write_file(
            &root,
            "application.properties",
            "next.application.name=base\nnext.config.import=/extra.properties\n",
        );
        write_file(&root, "extra.properties", "next.application.name=extra");
        let resource_loader = resource_loader(&root);
        let mut environment = MockEnvironment::default();

        process(&mut environment, &resource_loader).unwrap();

        assert_eq!(
            environment.source_names(),
            vec![
                "extra.properties".to_owned(),
                "application.properties".to_owned(),
            ]
        );
        assert_eq!(
            environment.property("next.application.name"),
            Some("extra".to_owned())
        );
    }

    #[test]
    fn locations_declared_by_the_environment_replace_the_default_locations() {
        let root = temp_root("location-property");
        write_file(
            &root,
            "application.properties",
            "next.application.name=default",
        );
        write_file(
            &root,
            "extra/application.properties",
            "next.application.name=extra",
        );
        let resource_loader = resource_loader(&root);
        let mut environment = MockEnvironment::default()
            .with_property_source("test", &[("next.config.location", "/extra/")]);

        process(&mut environment, &resource_loader).unwrap();

        assert_eq!(
            environment.source_names(),
            vec!["test".to_owned(), "extra/application.properties".to_owned(),]
        );
        assert_eq!(
            environment.property("next.application.name"),
            Some("extra".to_owned())
        );
    }

    #[test]
    fn additional_locations_take_precedence_over_the_default_locations() {
        let root = temp_root("additional-location");
        write_file(
            &root,
            "application.properties",
            "next.application.name=default",
        );
        write_file(
            &root,
            "extra/application.properties",
            "next.application.name=extra",
        );
        let resource_loader = resource_loader(&root);
        let mut environment = MockEnvironment::default()
            .with_property_source("test", &[("next.config.additional-location", "/extra/")]);

        process(&mut environment, &resource_loader).unwrap();

        assert_eq!(
            environment.source_names(),
            vec![
                "test".to_owned(),
                "extra/application.properties".to_owned(),
                "application.properties".to_owned(),
            ]
        );
        assert_eq!(
            environment.property("next.application.name"),
            Some("extra".to_owned())
        );
    }

    #[test]
    fn existing_property_sources_are_kept_and_take_precedence() {
        let root = temp_root("existing");
        write_file(
            &root,
            "application.properties",
            "next.application.name=config-data",
        );
        let resource_loader = resource_loader(&root);
        let mut environment = MockEnvironment::default().with_property_source(
            "commandLineArgs",
            &[("next.application.name", "command-line")],
        );

        process(&mut environment, &resource_loader).unwrap();

        assert_eq!(
            environment.source_names(),
            vec![
                "commandLineArgs".to_owned(),
                "application.properties".to_owned(),
            ]
        );
        assert_eq!(
            environment.property("next.application.name"),
            Some("command-line".to_owned())
        );
    }

    #[test]
    fn the_default_properties_source_keeps_the_lowest_precedence() {
        let root = temp_root("default-properties");
        write_file(
            &root,
            "application.properties",
            "next.application.name=config-data",
        );
        let resource_loader = resource_loader(&root);
        let mut environment = MockEnvironment::default().with_property_source(
            DefaultPropertiesPropertySource::NAME,
            &[("next.application.name", "default")],
        );

        process(&mut environment, &resource_loader).unwrap();

        assert_eq!(
            environment.source_names(),
            vec![
                "application.properties".to_owned(),
                DefaultPropertiesPropertySource::NAME.to_owned(),
            ]
        );
        assert_eq!(
            environment.property("next.application.name"),
            Some("config-data".to_owned())
        );
    }

    #[test]
    fn missing_mandatory_locations_fail() {
        let resource_loader = resource_loader(&temp_root("missing"));
        let mut environment = MockEnvironment::default()
            .with_property_source("test", &[("next.config.location", "/missing/")]);

        let error = process(&mut environment, &resource_loader).unwrap_err();

        assert!(matches!(
            error,
            ConfigDataEnvironmentError::LocationNotFound(_)
        ));
        assert_eq!(
            error.to_string(),
            "Config data location '/missing/' does not exist"
        );
    }

    #[test]
    fn missing_locations_can_be_ignored() {
        let root = temp_root("ignore-missing");
        write_file(
            &root,
            "application.properties",
            "next.application.name=demo",
        );
        let resource_loader = resource_loader(&root);
        let mut environment = MockEnvironment::default().with_property_source(
            "test",
            &[
                ("next.config.additional-location", "/missing/"),
                ("next.config.on-not-found", "ignore"),
            ],
        );

        process(&mut environment, &resource_loader).unwrap();

        assert_eq!(
            environment.property("next.application.name"),
            Some("demo".to_owned())
        );
    }

    #[test]
    fn missing_mandatory_imports_fail() {
        let root = temp_root("missing-import");
        write_file(
            &root,
            "application.properties",
            "next.config.import=/missing.properties\n",
        );
        let resource_loader = resource_loader(&root);
        let mut environment = MockEnvironment::default();

        let error = process(&mut environment, &resource_loader).unwrap_err();

        assert_eq!(
            error.to_string(),
            "Config data location '/missing.properties' does not exist"
        );
    }

    #[test]
    fn loads_file_system_locations() {
        let root = temp_root("file-system-location");
        write_file(
            &root,
            "application.properties",
            "next.application.name=resources",
        );
        let resource_loader = resource_loader(&root);

        let external = temp_root("file-system-external");
        write_file(
            &external,
            "application.properties",
            "next.application.name=file-system",
        );
        let location = format!("file:{}/", external.display()).replace('\\', "/");

        let mut environment = MockEnvironment::default()
            .with_property_source("test", &[("next.config.location", location.as_str())]);

        process(&mut environment, &resource_loader).unwrap();

        assert_eq!(
            environment.source_names(),
            vec![
                "test".to_owned(),
                format!("{location}application.properties"),
            ]
        );
        assert_eq!(
            environment.property("next.application.name"),
            Some("file-system".to_owned())
        );
    }

    #[test]
    fn profile_specific_documents_are_only_loaded_for_active_profiles() {
        let root = temp_root("inactive-profile");
        write_file(
            &root,
            "application.properties",
            "next.config.activate.on-profile=prod\nnext.application.name=prod\n",
        );
        let resource_loader = resource_loader(&root);
        let mut environment = MockEnvironment::default();

        process(&mut environment, &resource_loader).unwrap();

        assert_eq!(environment.property("next.application.name"), None);
    }

    #[test]
    fn included_profiles_are_activated_and_loaded() {
        let root = temp_root("included-profiles");
        write_file(
            &root,
            "application.properties",
            "next.profiles.include=local\nnext.application.name=base\n",
        );
        write_file(
            &root,
            "application-local.properties",
            "next.application.name=local\n",
        );
        let resource_loader = resource_loader(&root);
        let mut environment = MockEnvironment::default();

        process(&mut environment, &resource_loader).unwrap();

        assert_eq!(environment.active_profiles, vec!["local".to_owned()]);
        assert_eq!(
            environment.property("next.application.name"),
            Some("local".to_owned())
        );
    }

    #[test]
    fn additional_profiles_are_activated() {
        let root = temp_root("additional-profiles");
        write_file(
            &root,
            "application.properties",
            "next.application.name=base",
        );
        write_file(
            &root,
            "application-dev.properties",
            "next.application.name=dev\n",
        );
        let resource_loader = resource_loader(&root);
        let mut environment = MockEnvironment::default();

        let mut config_data = ConfigDataEnvironment::new(
            &mut environment,
            &resource_loader,
            ConfigDataLoader::default(),
            vec!["dev".to_owned()],
            &NoOpConfigDataEnvironmentUpdateListener,
        );
        config_data.process_and_apply().unwrap();

        assert_eq!(environment.active_profiles, vec!["dev".to_owned()]);
        assert_eq!(
            environment.property("next.application.name"),
            Some("dev".to_owned())
        );
    }

    #[test]
    fn notifies_the_update_listener() {
        let root = temp_root("update-listener");
        write_file(
            &root,
            "application.properties",
            "next.profiles.active=dev\nnext.application.name=base\n",
        );
        let resource_loader = resource_loader(&root);
        let mut environment = MockEnvironment::default();
        let listener = RecordingUpdateListener::default();

        let mut config_data = ConfigDataEnvironment::new(
            &mut environment,
            &resource_loader,
            ConfigDataLoader::default(),
            Vec::new(),
            &listener,
        );
        config_data.process_and_apply().unwrap();

        assert_eq!(
            listener.added.lock().unwrap().clone(),
            vec!["application.properties".to_owned()]
        );
        assert_eq!(
            listener.profiles.lock().unwrap().clone(),
            vec!["dev".to_owned()]
        );
    }

    #[test]
    fn rejects_invalid_properties_in_profile_specific_documents() {
        let root = temp_root("invalid-properties");
        write_file(
            &root,
            "application.properties",
            "next.profiles.active=dev\n",
        );
        write_file(
            &root,
            "application-dev.properties",
            "next.profiles.active=prod\n",
        );
        let resource_loader = resource_loader(&root);
        let mut environment = MockEnvironment::default();

        let error = process(&mut environment, &resource_loader).unwrap_err();

        assert!(matches!(
            error,
            ConfigDataEnvironmentError::InvalidProperty(_)
        ));
    }

    #[test]
    fn reads_the_not_found_action_from_the_environment() {
        let resource_loader = resource_loader(&temp_root("not-found-action"));
        let mut environment = MockEnvironment::default().with_property_source(
            "test",
            &[
                ("next.config.location", "/missing/"),
                ("next.config.on-not-found", "IGNORE"),
            ],
        );

        assert!(process(&mut environment, &resource_loader).is_ok());
    }
}
