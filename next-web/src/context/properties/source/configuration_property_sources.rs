//! Provides access to
//! [`ConfigurationPropertySource`](crate::context::properties::source::ConfigurationPropertySource)
//! instances.

use next_web_core::env::{
    ConfigurableEnvironment, MutablePropertySources, PropertySource, PropertySourceValue,
};

use crate::context::properties::source::{
    ConfigurationPropertySourcesPropertyResolver, ConfigurationPropertySourcesPropertySource,
    NextConfigurationPropertySource, NextConfigurationPropertySources,
};

/// The name of the property source adapter attached by
/// [`ConfigurationPropertySources::attach`].
const ATTACHED_PROPERTY_SOURCE_NAME: &str = "configurationProperties";

/// Provides access to
/// [`ConfigurationPropertySource`](crate::context::properties::source::ConfigurationPropertySource)
/// instances.
///
/// This is the Rust equivalent of Next Boot's `ConfigurationPropertySources`.
pub struct ConfigurationPropertySources;

impl ConfigurationPropertySources {
    /// Creates a new property resolver that resolves property values against an
    /// underlying set of property sources.
    ///
    /// Provides an
    /// [`ConfigurationPropertySource`](crate::context::properties::source::ConfigurationPropertySource)
    /// aware and optimized alternative to a plain property sources resolver.
    ///
    /// # Arguments
    ///
    /// * `property_sources` - The property sources to resolve against.
    pub fn create_property_resolver(
        property_sources: &MutablePropertySources,
    ) -> ConfigurationPropertySourcesPropertyResolver {
        ConfigurationPropertySourcesPropertyResolver::new(NextConfigurationPropertySources::new(
            property_sources,
        ))
    }

    /// Determines whether the given property source is the configuration
    /// property source that was attached to an environment.
    ///
    /// # Arguments
    ///
    /// * `property_source` - The property source to test.
    pub fn is_attached_configuration_property_source<T>(
        property_source: &dyn PropertySource<T>,
    ) -> bool {
        property_source.name() == ATTACHED_PROPERTY_SOURCE_NAME
    }

    /// Attaches configuration property source support to the given environment.
    ///
    /// Each property source managed by the environment is adapted so that
    /// classic property resolution can resolve values using configuration
    /// property names. The adapter is added with the highest precedence.
    ///
    /// Note: Java reuses the attached adapter when it already adapts the same
    /// property sources, and tracks additions or removals from the environment.
    /// The adapter snapshots the property sources here, so it is rebuilt and
    /// moved to the front on every call.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment to attach to. Rust enforces that this
    ///   is a [`ConfigurableEnvironment`], where Java asserts the same.
    pub fn attach(environment: &mut dyn ConfigurableEnvironment) {
        let sources = environment.property_sources();

        sources.remove(ATTACHED_PROPERTY_SOURCE_NAME);

        let adapted = NextConfigurationPropertySources::new(sources);
        sources.add_first(Box::new(ConfigurationPropertySourcesPropertySource::new(
            ATTACHED_PROPERTY_SOURCE_NAME,
            adapted,
        )));
    }

    /// Returns the property source that has been
    /// [attached](Self::attach) to the given property sources, if any.
    ///
    /// # Arguments
    ///
    /// * `sources` - The property sources to inspect.
    pub fn get_attached(
        sources: &MutablePropertySources,
    ) -> Option<&dyn PropertySource<PropertySourceValue>> {
        sources.get(ATTACHED_PROPERTY_SOURCE_NAME)
    }

    /// Returns the configuration property sources of the given environment.
    ///
    /// The property sources are adapted from the environment, skipping stub
    /// property sources and any attached adapter. Java returns the sources of
    /// the attached adapter when there is one; adapting the environment's
    /// sources again yields the same result, so the same pipeline is used for
    /// both cases here.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment to read the property sources from.
    pub fn get(
        environment: &mut dyn ConfigurableEnvironment,
    ) -> NextConfigurationPropertySources {
        NextConfigurationPropertySources::new(environment.property_sources())
    }

    /// Returns a collection containing a single new configuration property
    /// source adapted from the given property source.
    ///
    /// The single element is `None` when the source cannot be adapted.
    ///
    /// # Arguments
    ///
    /// * `source` - The property source to adapt.
    pub fn from(
        source: &dyn PropertySource<PropertySourceValue>,
    ) -> Vec<Option<NextConfigurationPropertySource>> {
        vec![NextConfigurationPropertySource::from(source)]
    }

    /// Returns a collection of new configuration property sources adapted from
    /// the given property sources.
    ///
    /// Property sources that are not included, such as stub property sources
    /// and any attached adapter, are filtered out.
    ///
    /// # Arguments
    ///
    /// * `sources` - The property sources to adapt.
    pub fn from_sources(sources: &MutablePropertySources) -> NextConfigurationPropertySources {
        NextConfigurationPropertySources::new(sources)
    }
}

/// Returns the property sources of the given collection that are adapted to
/// configuration property sources.
///
/// # Arguments
///
/// * `sources` - The property sources to adapt.
pub(crate) fn stream_property_sources(
    sources: &MutablePropertySources,
) -> Vec<&dyn PropertySource<PropertySourceValue>> {
    sources
        .iter()
        .flat_map(flatten)
        .filter(|source| is_included(*source))
        .collect()
}

/// Returns the given source, or the sources of the environment it wraps.
///
/// Note: Java unwraps the source object of a property source when it is a
/// [`ConfigurableEnvironment`]. Rust property sources always expose their
/// properties directly, so there is nothing to unwrap and the source itself is
/// returned.
///
/// # Arguments
///
/// * `source` - The property source to flatten.
fn flatten(
    source: &dyn PropertySource<PropertySourceValue>,
) -> Vec<&dyn PropertySource<PropertySourceValue>> {
    vec![source]
}

/// Returns whether the given source should be adapted.
///
/// Stub property sources and the attached adapter are not included.
///
/// # Arguments
///
/// * `source` - The property source to test.
fn is_included(source: &dyn PropertySource<PropertySourceValue>) -> bool {
    !source.is_stub() && source.name() != ATTACHED_PROPERTY_SOURCE_NAME
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{context::ApplicationEnvironment, env::MapPropertySource};
    use next_web_core::util::indexmap::IndexMap;

    /// Creates an environment holding a single application property.
    fn environment() -> ApplicationEnvironment {
        let mut environment = ApplicationEnvironment::default();

        let mut properties = IndexMap::new();
        properties.insert("next.application.name".to_owned(), "demo".to_owned());
        environment
            .property_sources()
            .add_last(Box::new(MapPropertySource::new(
                "test".to_owned(),
                properties,
            )));

        environment
    }

    /// Returns the names of the property sources of the given environment.
    fn names(environment: &mut ApplicationEnvironment) -> Vec<String> {
        environment
            .property_sources()
            .iter()
            .map(|source| source.name().to_owned())
            .collect()
    }

    #[test]
    fn attaches_the_adapter_with_the_highest_precedence() {
        let mut environment = environment();

        ConfigurationPropertySources::attach(&mut environment);

        assert_eq!(
            names(&mut environment),
            vec!["configurationProperties", "test"]
        );

        let attached = ConfigurationPropertySources::get_attached(environment.property_sources())
            .expect("the adapter should be attached");
        assert!(ConfigurationPropertySources::is_attached_configuration_property_source(attached));
    }

    #[test]
    fn attaching_twice_keeps_a_single_adapter() {
        let mut environment = environment();

        ConfigurationPropertySources::attach(&mut environment);
        ConfigurationPropertySources::attach(&mut environment);

        assert_eq!(
            names(&mut environment),
            vec!["configurationProperties", "test"]
        );
    }

    #[test]
    fn the_adapter_resolves_adapted_properties() {
        let mut environment = environment();
        ConfigurationPropertySources::attach(&mut environment);

        let attached = ConfigurationPropertySources::get_attached(environment.property_sources())
            .expect("the adapter should be attached");

        assert_eq!(
            attached.property("next.application.name"),
            Some("demo".to_owned())
        );
        assert!(attached.contains_property("next.application.name"));
        assert_eq!(attached.property("next.application.version"), None);
    }

    #[test]
    fn the_adapter_is_not_adapted_again() {
        let mut environment = environment();
        ConfigurationPropertySources::attach(&mut environment);

        let sources = ConfigurationPropertySources::get(&mut environment);

        // Only the "test" source remains, the adapter itself is filtered out.
        assert_eq!(sources.len(), 1);
    }

    #[test]
    fn adapts_a_single_property_source() {
        let mut environment = environment();
        let source = environment
            .property_sources()
            .get("test")
            .expect("the test source should be present");

        let adapted = ConfigurationPropertySources::from(source);

        assert_eq!(adapted.len(), 1);
        assert!(adapted[0].is_some());
    }

    #[test]
    fn adapts_property_sources_of_an_environment() {
        let mut environment = environment();

        let adapted = ConfigurationPropertySources::from_sources(environment.property_sources());

        assert_eq!(adapted.len(), 1);
        assert!(adapted.is_using_sources(environment.property_sources()));
    }

    #[test]
    fn creates_a_property_resolver() {
        let mut environment = environment();
        let resolver =
            ConfigurationPropertySources::create_property_resolver(environment.property_sources());

        assert_eq!(
            resolver.get_property("next.application.name"),
            Some("demo".to_owned())
        );
        assert_eq!(
            resolver.resolve_placeholders("the ${next.application.name} application"),
            "the demo application"
        );
    }
}
