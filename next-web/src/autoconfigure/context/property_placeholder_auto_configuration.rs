use std::error::Error;
use std::fmt;
use std::sync::Arc;

use next_web_context::{
    support::{
        PropertySource, PropertySourcesPlaceholderConfigurer,
        ENVIRONMENT_PROPERTIES_PROPERTY_SOURCE_NAME,
    },
    ApplicationContextExt,
};
use next_web_core::env::ConfigurableEnvironment;
use next_web_core::ApplicationContext;
use next_web_macros::auto_configuration;

/// Auto-configuration for PropertySourcesPlaceholderConfigurer.
pub struct PropertyPlaceholderAutoConfiguration;

#[auto_configuration]
impl PropertyPlaceholderAutoConfiguration {
    /// Creates the placeholder configurer of the application.
    ///
    /// The configurer resolves its placeholders against the environment of the
    /// context, which is registered under
    /// [`APPLICATION_ENVIRONMENT_SINGLETON_NAME`](next_web_context::APPLICATION_ENVIRONMENT_SINGLETON_NAME)
    /// before the auto-configurations run. The configurer is applied right away,
    /// so that the property sources it uses are recorded and available through
    /// [`applied_property_sources`](PropertySourcesPlaceholderConfigurer::applied_property_sources).
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment of the context.
    ///
    /// # Errors
    ///
    /// Returns an error when the configurer cannot be applied.
    #[provider(
        // The name is the default singleton name of the configurer, which is
        // what `conditional_on_missing_singleton` looks for.
        name = "propertySourcesPlaceholderConfigurer",
        conditional = [Self::conditional_on_missing_singleton],
        order = i32::MIN
    )]
    pub fn property_sources_placeholder_configurer(
        #[autowired(name = "applicationEnvironment")] environment: Arc<dyn ConfigurableEnvironment>,
    ) -> Result<PropertySourcesPlaceholderConfigurer, Box<dyn Error>> {
        let mut configurer = PropertySourcesPlaceholderConfigurer::default();
        configurer.set_environment(Arc::new(EnvironmentPropertySource::new(environment)));
        configurer.apply()?;

        Ok(configurer)
    }

    /// Returns whether the configurer is not registered yet.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context the configurer is looked up in.
    pub fn conditional_on_missing_singleton(ctx: &dyn ApplicationContext) -> bool {
        !ctx.contains_singleton_with_default_name::<PropertySourcesPlaceholderConfigurer>()
    }
}

/// Adapts the environment of the context to the [`PropertySource`] the
/// placeholder configurer resolves its placeholders with.
///
/// The environment resolves the placeholders of the values it holds, so a
/// configuration value such as `the ${next.name} application` is resolved by the
/// environment before the configurer sees it, exactly like a
/// `ConfigurableEnvironmentPropertySource` does.
struct EnvironmentPropertySource(Arc<dyn ConfigurableEnvironment>);

impl EnvironmentPropertySource {
    /// Creates a property source over the given environment.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment the properties are read from.
    fn new(environment: Arc<dyn ConfigurableEnvironment>) -> Self {
        Self(environment)
    }
}

impl PropertySource for EnvironmentPropertySource {
    fn name(&self) -> &str {
        ENVIRONMENT_PROPERTIES_PROPERTY_SOURCE_NAME
    }

    fn property(&self, name: &str) -> Option<String> {
        self.0.get_property(name)
    }

    fn contains_property(&self, name: &str) -> bool {
        self.0.contains_property(name)
    }
}

impl fmt::Debug for EnvironmentPropertySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EnvironmentPropertySource")
            .field("active_profiles", &self.0.active_profiles())
            .finish()
    }
}
