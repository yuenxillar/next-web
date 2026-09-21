//! Imports config data by resolving locations and loading their resources.

use std::collections::{BTreeSet, HashSet};

use next_web_core::io::ResourceLoader;
use tracing::{debug, trace};

use crate::context::config::config_data_activation_context::ConfigDataActivationContext;
use crate::context::config::config_data_environment_contributor::ConfigDataEnvironmentContributor;
use crate::context::config::config_data_loader::ConfigDataLoader;
use crate::context::config::config_data_location::ConfigDataLocation;
use crate::context::config::config_data_location_not_found_error::ConfigDataLocationNotFoundError;
use crate::context::config::config_data_not_found_action::ConfigDataNotFoundAction;
use crate::context::config::config_data_resource::ConfigDataLocationResolver;

/// Imports config data by resolving locations and loading their resources.
///
/// The importer keeps track of the resources it has loaded, so that a resource
/// is only loaded once, and of the locations it has seen, so that the locations
/// that were loaded can be reported back to the environment.
pub(crate) struct ConfigDataImporter<'a> {
    resource_loader: &'a dyn ResourceLoader,
    resolver: ConfigDataLocationResolver,
    loader: ConfigDataLoader,
    not_found_action: ConfigDataNotFoundAction,
    resolved_resources: HashSet<String>,
    loaded_locations: BTreeSet<ConfigDataLocation>,
    optional_locations: BTreeSet<ConfigDataLocation>,
}

impl<'a> ConfigDataImporter<'a> {
    /// Creates a new importer.
    ///
    /// # Arguments
    ///
    /// * `resource_loader` - The loader used for locations without a `file:`
    ///   prefix.
    /// * `loader` - The loader used for the resources of a location.
    /// * `not_found_action` - The action to take when a location is missing.
    pub(crate) fn new(
        resource_loader: &'a dyn ResourceLoader,
        loader: ConfigDataLoader,
        not_found_action: ConfigDataNotFoundAction,
    ) -> Self {
        let resolver = ConfigDataLocationResolver::new(loader.extensions());

        Self {
            resource_loader,
            resolver,
            loader,
            not_found_action,
            resolved_resources: HashSet::new(),
            loaded_locations: BTreeSet::new(),
            optional_locations: BTreeSet::new(),
        }
    }

    /// Returns the locations that were loaded.
    pub(crate) fn loaded_locations(&self) -> &BTreeSet<ConfigDataLocation> {
        &self.loaded_locations
    }

    /// Returns the optional locations that were missing.
    pub(crate) fn optional_locations(&self) -> &BTreeSet<ConfigDataLocation> {
        &self.optional_locations
    }

    /// Resolves and loads the given location.
    ///
    /// The returned contributors are ordered by descending precedence and hold
    /// the documents that were loaded from the location. Resources that have
    /// already been loaded, for example because the method is called a second
    /// time with the profiles in place, are not loaded again.
    ///
    /// # Arguments
    ///
    /// * `location` - The location to resolve and load.
    /// * `activation_context` - The context holding the active profiles.
    ///
    /// # Errors
    ///
    /// Returns an error when the location is missing and the
    /// `next.config.on-not-found` property asks for missing locations to fail.
    pub(crate) fn resolve_and_load(
        &mut self,
        location: &ConfigDataLocation,
        activation_context: &ConfigDataActivationContext,
    ) -> Result<Vec<ConfigDataEnvironmentContributor>, ConfigDataLocationNotFoundError> {
        let resolution = self.resolver.resolve(
            location,
            activation_context.active_profiles(),
            self.resource_loader,
        );

        if !resolution.is_found() {
            debug!("Config data location '{location}' does not exist");
            if location.is_optional() {
                self.optional_locations.insert(location.clone());
                return Ok(Vec::new());
            }
            self.loaded_locations.insert(location.clone());
            self.not_found_action
                .handle(ConfigDataLocationNotFoundError::new(location.clone()))?;
            return Ok(Vec::new());
        }

        self.loaded_locations.insert(location.clone());

        let mut contributors = Vec::new();
        for resource in resolution.resources() {
            if !self.resolved_resources.insert(resource.key()) {
                continue;
            }

            let config_data = match self.loader.load(resource, self.resource_loader) {
                Ok(config_data) => config_data,
                Err(error) => {
                    debug!("Unable to load config data from '{resource}': {error}");
                    self.not_found_action
                        .handle(ConfigDataLocationNotFoundError::new(
                            resource.location().clone(),
                        ))?;
                    continue;
                }
            };

            for config in config_data {
                if let Some(expression) = config.activate_profile() {
                    if !activation_context.accepts(&[expression]) {
                        trace!(
                            "Skipping inactive config data '{}' ({})",
                            config.property_source().name(),
                            expression
                        );
                        continue;
                    }
                }

                contributors.push(ConfigDataEnvironmentContributor::of_config_data(
                    resource.location().clone(),
                    config,
                    resource.profile().is_some(),
                ));
            }
        }

        Ok(contributors)
    }
}
