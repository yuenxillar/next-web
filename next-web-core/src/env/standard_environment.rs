//! An environment for standard applications.

use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::env::{
    BaseEnvironment, ConfigurableEnvironment, MapPropertySource, PropertyLookup,
    PropertySourcesLookup,
};
use crate::impl_environment_delegate;
use crate::util::indexmap::IndexMap;

/// Name of the property source that holds the environment variables of the
/// process.
pub const SYSTEM_ENVIRONMENT_PROPERTY_SOURCE_NAME: &str = "systemEnvironment";

/// Name of the property source that holds the system properties.
///
/// Rust has no process wide property table, so an environment does not add a
/// property source with this name unless it provides one of its own.
pub const SYSTEM_PROPERTIES_PROPERTY_SOURCE_NAME: &str = "systemProperties";

/// An [`Environment`](crate::env::Environment) that is suitable for a standard
/// (non web) application.
///
/// The environment adds the property sources of the process to the ones of a
/// [`BaseEnvironment`], which are searched after the property sources an
/// application adds itself:
///
/// 1. the environment variables of the process
///
/// Note: Rust has no process wide property table, so only the environment
/// variables of the process are added. An environment that has a property table
/// of its own adds a source with the
/// [`SYSTEM_PROPERTIES_PROPERTY_SOURCE_NAME`] name before the environment
/// variables, so that it takes precedence over them.
///
/// The property sources of the process can be removed, reordered or replaced
/// through the [`MutablePropertySources`](crate::env::MutablePropertySources) of
/// the environment, which is how an application customizes the properties it
/// searches.
pub struct StandardEnvironment {
    base: BaseEnvironment,
}

impl StandardEnvironment {
    /// Creates an environment with the property sources of a standard
    /// application.
    pub fn new() -> Self {
        Self::with_lookup(Box::new(PropertySourcesLookup), true)
    }

    /// Creates an environment with the property sources of a standard
    /// application that looks its properties up with the given strategy.
    ///
    /// # Arguments
    ///
    /// * `lookup` - The strategy used to look the properties up.
    /// * `read_profile_properties` - Whether the active and default profiles may
    ///   be taken from the properties that declare them.
    pub fn with_lookup(lookup: Box<dyn PropertyLookup>, read_profile_properties: bool) -> Self {
        let mut base = BaseEnvironment::with_lookup(lookup, read_profile_properties);

        let system_environment = to_index_map(base.system_environment());
        base.property_sources()
            .add_last(Box::new(MapPropertySource::new(
                SYSTEM_ENVIRONMENT_PROPERTY_SOURCE_NAME.to_owned(),
                system_environment,
            )));

        Self { base }
    }
}

impl Default for StandardEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for StandardEnvironment {
    type Target = BaseEnvironment;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for StandardEnvironment {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl_environment_delegate!(StandardEnvironment, base);

/// Returns the given properties as an insertion ordered map with sorted keys,
/// so that the property names of the source are deterministic.
///
/// # Arguments
///
/// * `properties` - The properties to order.
fn to_index_map(properties: HashMap<String, String>) -> IndexMap<String, String> {
    let mut properties: Vec<(String, String)> = properties.into_iter().collect();
    properties.sort_by(|(left, _), (right, _)| left.cmp(right));
    properties.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use crate::env::{Environment, PropertyResolver};

    use super::*;

    #[test]
    fn adds_the_environment_variables_of_the_process() {
        let environment = StandardEnvironment::new();

        let names: Vec<&str> = environment
            .property_sources_ref()
            .iter()
            .map(|property_source| property_source.name())
            .collect();
        assert_eq!(names, vec![SYSTEM_ENVIRONMENT_PROPERTY_SOURCE_NAME]);
        assert_eq!(
            environment.system_environment(),
            std::env::vars().collect::<HashMap<String, String>>()
        );
        assert!(environment.system_properties().is_empty());
    }

    #[test]
    fn searches_the_property_sources_of_the_application_first() {
        let mut environment = StandardEnvironment::new();
        let path = environment.system_environment().get("PATH").cloned();
        let Some(path) = path else {
            // A process without a PATH has nothing to override.
            return;
        };

        environment
            .property_sources()
            .add_first(Box::new(MapPropertySource::new(
                "test".to_owned(),
                IndexMap::from([("PATH".to_owned(), "overridden".to_owned())]),
            )));

        assert_eq!(
            environment.get_property("PATH"),
            Some("overridden".to_owned())
        );
        assert_ne!(environment.get_property("PATH"), Some(path));
    }

    #[test]
    fn resolves_the_profiles_from_the_profile_properties() {
        let mut environment = StandardEnvironment::new();
        environment
            .property_sources()
            .add_first(Box::new(MapPropertySource::new(
                "test".to_owned(),
                IndexMap::from([(
                    crate::env::ACTIVE_PROFILES_PROPERTY_NAME.to_owned(),
                    "dev, cloud".to_owned(),
                )]),
            )));

        assert_eq!(
            environment.active_profiles(),
            &["dev".to_owned(), "cloud".to_owned()]
        );

        environment.set_active_profiles(&["local"]);

        assert_eq!(environment.active_profiles(), &["local".to_owned()]);
        assert_eq!(
            environment.default_profiles(),
            &[crate::env::RESERVED_DEFAULT_PROFILE_NAME.to_owned()]
        );
    }

    #[test]
    fn creates_a_default_environment() {
        let environment = StandardEnvironment::default();

        assert_eq!(environment.property_sources_ref().len(), 1);
    }
}
