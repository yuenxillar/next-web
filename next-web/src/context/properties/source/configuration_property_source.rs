//! A source of configuration properties.

use crate::context::properties::source::{ConfigurationProperty, ConfigurationPropertyName};

/// A source of configuration properties, addressed by
/// [`ConfigurationPropertyName`].
///
/// This is the Rust equivalent of Next Boot's `ConfigurationPropertySource`.
/// The static `from(PropertySource)` factory is provided by
/// [`NextConfigurationPropertySource::from`], since Rust traits cannot declare
/// factory methods that return a specific implementation.
///
/// [`NextConfigurationPropertySource::from`]: crate::context::properties::source::NextConfigurationPropertySource::from
pub trait ConfigurationPropertySource {
    /// Returns the configuration property with the given name, or `None` when
    /// this source does not contain such a property.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the configuration property.
    fn get_configuration_property(
        &self,
        name: &ConfigurationPropertyName,
    ) -> Option<ConfigurationProperty>;
}
