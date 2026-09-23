//! An [`EnvironmentPostProcessor`] that loads and applies config data to the
//! environment.

use std::sync::Arc;

use next_web_core::{
    env::ConfigurableEnvironment,
    io::{DefaultResourceLoader, ResourceLoader},
    Ordered,
};
use tracing::error;

use crate::context::config::{
    ConfigDataEnvironment, ConfigDataEnvironmentError, ConfigDataEnvironmentUpdateListener,
    ConfigDataLoader, NoOpConfigDataEnvironmentUpdateListener, ON_NOT_FOUND_PROPERTY,
};
use crate::EnvironmentPostProcessor;

/// Property used to determine what action to take when a config data location
/// that must exist cannot be found.
pub const ON_LOCATION_NOT_FOUND_PROPERTY: &str = ON_NOT_FOUND_PROPERTY;

/// An [`EnvironmentPostProcessor`] that loads and applies config data to the
/// environment before the application context is refreshed.
///
/// The locations that are imported are taken from the `next.config.import`,
/// `next.config.additional-location` and `next.config.location` properties of
/// the environment. When `next.config.location` is not set, the default search
/// locations are used.
#[derive(Default)]
pub struct ConfigDataEnvironmentPostProcessor {
    resource_loader: Option<Arc<dyn ResourceLoader>>,
    loader: ConfigDataLoader,
    additional_profiles: Vec<String>,
    update_listener: Option<Box<dyn ConfigDataEnvironmentUpdateListener>>,
}

impl ConfigDataEnvironmentPostProcessor {
    /// The order of the processor.
    ///
    /// The processor runs just after the highest precedence, so that the
    /// properties it contributes are available to the processors that run after
    /// it.
    pub const ORDER: i32 = i32::MIN + 10;

    /// Sets the resource loader that is used to resolve the locations of the
    /// resources.
    ///
    /// # Arguments
    ///
    /// * `resource_loader` - The loader to use.
    pub fn set_resource_loader(&mut self, resource_loader: Arc<dyn ResourceLoader>) {
        self.resource_loader = Some(resource_loader);
    }

    /// Sets the resource loader that is used to resolve the locations of the
    /// resources, and returns the processor.
    ///
    /// # Arguments
    ///
    /// * `resource_loader` - The loader to use.
    pub fn with_resource_loader(mut self, resource_loader: Arc<dyn ResourceLoader>) -> Self {
        self.set_resource_loader(resource_loader);
        self
    }

    /// Sets the profiles to activate on top of the profiles that are declared
    /// by the environment and by the config data.
    ///
    /// # Arguments
    ///
    /// * `additional_profiles` - The profiles to activate.
    pub fn set_additional_profiles(&mut self, additional_profiles: Vec<String>) {
        self.additional_profiles = additional_profiles;
    }

    /// Sets the profiles to activate on top of the profiles that are declared
    /// by the environment and by the config data, and returns the processor.
    ///
    /// # Arguments
    ///
    /// * `additional_profiles` - The profiles to activate.
    pub fn with_additional_profiles(mut self, additional_profiles: Vec<String>) -> Self {
        self.set_additional_profiles(additional_profiles);
        self
    }

    /// Sets the listener that is notified of the updates that are made to the
    /// environment.
    ///
    /// # Arguments
    ///
    /// * `update_listener` - The listener to use.
    pub fn set_update_listener(
        &mut self,
        update_listener: Box<dyn ConfigDataEnvironmentUpdateListener>,
    ) {
        self.update_listener = Some(update_listener);
    }

    /// Sets the listener that is notified of the updates that are made to the
    /// environment, and returns the processor.
    ///
    /// # Arguments
    ///
    /// * `update_listener` - The listener to use.
    pub fn with_update_listener(
        mut self,
        update_listener: Box<dyn ConfigDataEnvironmentUpdateListener>,
    ) -> Self {
        self.set_update_listener(update_listener);
        self
    }

    /// Applies config data to the given environment.
    ///
    /// The environment is left unchanged when an error is returned.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment to apply config data to.
    ///
    /// # Errors
    ///
    /// Returns an error when a location that must exist cannot be found, or
    /// when the config data holds a property that is not allowed.
    pub fn apply(
        &self,
        environment: &mut dyn ConfigurableEnvironment,
    ) -> Result<(), ConfigDataEnvironmentError> {
        let update_listener: &dyn ConfigDataEnvironmentUpdateListener = match &self.update_listener
        {
            Some(update_listener) => update_listener.as_ref(),
            None => &NoOpConfigDataEnvironmentUpdateListener,
        };

        let mut config_data = ConfigDataEnvironment::new(
            environment,
            self.resource_loader(),
            self.loader.clone(),
            self.additional_profiles.clone(),
            update_listener,
        );

        config_data.process_and_apply()
    }

    /// Applies config data to the given environment, using the given resource
    /// loader, profiles and listener.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment to apply config data to.
    /// * `resource_loader` - The loader used to resolve the locations of the
    ///   resources.
    /// * `additional_profiles` - Profiles to activate on top of the profiles
    ///   that are declared by the environment and by the config data.
    /// * `update_listener` - Listener notified of the updates that are made.
    ///
    /// # Errors
    ///
    /// Returns an error when a location that must exist cannot be found, or
    /// when the config data holds a property that is not allowed.
    pub fn apply_to_with(
        environment: &mut dyn ConfigurableEnvironment,
        resource_loader: &dyn ResourceLoader,
        additional_profiles: &[String],
        update_listener: &dyn ConfigDataEnvironmentUpdateListener,
    ) -> Result<(), ConfigDataEnvironmentError> {
        let mut config_data = ConfigDataEnvironment::new(
            environment,
            resource_loader,
            ConfigDataLoader::default(),
            additional_profiles.to_vec(),
            update_listener,
        );

        config_data.process_and_apply()
    }

    /// Applies config data to the given environment, using the default resource
    /// loader and no additional profiles.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment to apply config data to.
    ///
    /// # Errors
    ///
    /// Returns an error when a location that must exist cannot be found, or
    /// when the config data holds a property that is not allowed.
    pub fn apply_to(
        environment: &mut dyn ConfigurableEnvironment,
    ) -> Result<(), ConfigDataEnvironmentError> {
        Self::default().apply(environment)
    }

    /// Returns the resource loader of this processor, or the default one when
    /// none has been set.
    fn resource_loader(&self) -> &dyn ResourceLoader {
        match &self.resource_loader {
            Some(resource_loader) => resource_loader.as_ref(),
            // The shared loader is created and loaded once per process, so the
            // resources directory is read once even when the banner is resolved
            // through the same loader.
            None => DefaultResourceLoader::shared(),
        }
    }
}

impl EnvironmentPostProcessor for ConfigDataEnvironmentPostProcessor {
    /// Applies config data to the given environment.
    ///
    /// The trait cannot report an error, so a failure is logged and the
    /// environment is left unchanged. Use
    /// [`ConfigDataEnvironmentPostProcessor::apply`] to handle failures.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment to apply config data to.
    fn post_process_environment(&mut self, environment: &mut dyn ConfigurableEnvironment) {
        if let Err(error) = self.apply(environment) {
            error!("Config data could not be applied: {error}");
        }
    }
}

impl Ordered for ConfigDataEnvironmentPostProcessor {
    fn order(&self) -> i32 {
        Self::ORDER
    }
}
