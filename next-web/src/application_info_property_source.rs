//! A [`PropertySource`] which provides information about the application, like
//! the process ID (PID) or the version.

use next_web_core::{
    env::{ConfigurableEnvironment, PropertySource},
    util::{indexmap::IndexMap, StringUtils},
};

use crate::{
    env::{MapPropertySource, PropertySourceInfo},
    NextWebVersion,
};

/// The name of the `applicationInfo` property source.
pub const APPLICATION_INFO_PROPERTY_SOURCE_NAME: &str = "applicationInfo";

/// The property holding the version of the application.
const APPLICATION_VERSION_PROPERTY: &str = "next.application.version";

/// The property holding the process ID of the application.
const APPLICATION_PID_PROPERTY: &str = "next.application.pid";

/// A [`PropertySource`] which provides information about the application, like
/// the process ID (PID) or the version.
///
/// The source is immutable: the information it exposes is resolved once, when
/// the source is created.
#[derive(Debug)]
pub struct ApplicationInfoPropertySource {
    delegate: MapPropertySource,
}

impl ApplicationInfoPropertySource {
    /// The name of the `applicationInfo` property source.
    pub const NAME: &'static str = APPLICATION_INFO_PROPERTY_SOURCE_NAME;

    /// Creates a new source exposing the process ID of the running
    /// application.
    ///
    /// The version of the application is not exposed, since it is not known
    /// when the source is created this way.
    pub fn new() -> Self {
        Self::with_application_version(None)
    }

    /// Creates a new source exposing the given application version.
    ///
    /// A version that is absent, empty or blank is not exposed.
    ///
    /// # Arguments
    ///
    /// * `application_version` - The version of the application, when known.
    pub fn with_application_version(application_version: Option<&str>) -> Self {
        Self {
            delegate: MapPropertySource::new(
                Self::NAME.to_owned(),
                Self::get_properties(application_version),
            ),
        }
    }

    /// Creates a new source reading the application version from the given main
    /// application class.
    ///
    /// Java resolves the version from the implementation version of the package
    /// hosting the main class. A compiled Rust application does not carry such a
    /// version, so the version of the framework compiled into the application is
    /// used when a main class is known. Passing `None` is the equivalent of a
    /// `null` main class and never exposes a version.
    ///
    /// # Arguments
    ///
    /// * `main_class` - The name of the main application class, when known.
    pub fn from_main_class(main_class: Option<&str>) -> Self {
        Self::with_application_version(Self::read_version(main_class).as_deref())
    }

    /// Moves the [`ApplicationInfoPropertySource`] to the end of the
    /// environment's property sources.
    ///
    /// Does nothing when the environment does not contain an `applicationInfo`
    /// property source.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment to update.
    pub fn move_to_end(environment: &mut dyn ConfigurableEnvironment) {
        let property_sources = environment.property_sources();
        if let Some(property_source) = property_sources.remove(Self::NAME) {
            property_sources.add_last(property_source);
        }
    }

    /// Returns the properties exposed by this source.
    ///
    /// # Arguments
    ///
    /// * `application_version` - The version of the application, when known.
    fn get_properties(application_version: Option<&str>) -> IndexMap<String, String> {
        let mut result = IndexMap::new();

        if let Some(application_version) =
            application_version.filter(|version| StringUtils::has_text(version))
        {
            result.insert(
                APPLICATION_VERSION_PROPERTY.to_owned(),
                application_version.to_owned(),
            );
        }

        if let Some(pid) = application_pid() {
            result.insert(APPLICATION_PID_PROPERTY.to_owned(), pid.to_string());
        }

        result
    }

    /// Reads the application version of the given main application class.
    ///
    /// # Arguments
    ///
    /// * `main_class` - The name of the main application class, when known.
    fn read_version(main_class: Option<&str>) -> Option<String> {
        main_class
            .filter(|name| StringUtils::has_text(name))
            .map(|_| NextWebVersion::get_version().to_owned())
    }
}

impl Default for ApplicationInfoPropertySource {
    /// Creates a source that exposes the process ID of the running application.
    fn default() -> Self {
        Self::new()
    }
}

/// Returns the process ID of the running application, when it can be
/// determined.
fn application_pid() -> Option<u32> {
    Some(std::process::id())
}

impl PropertySourceInfo for ApplicationInfoPropertySource {
    /// Always returns `true`.
    fn is_immutable(&self) -> bool {
        true
    }
}

impl PropertySource<IndexMap<String, String>> for ApplicationInfoPropertySource {
    fn name(&self) -> &str {
        self.delegate.name()
    }

    fn property(&self, name: &str) -> Option<String> {
        self.delegate.property(name)
    }

    fn contains_property(&self, name: &str) -> bool {
        self.delegate.contains_property(name)
    }

    fn property_names(&self) -> Vec<String> {
        self.delegate.property_names()
    }

    fn source(&self) -> &IndexMap<String, String> {
        self.delegate.source()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ApplicationEnvironment;

    fn pid() -> String {
        std::process::id().to_string()
    }

    #[test]
    fn exposes_application_info_properties() {
        let source = ApplicationInfoPropertySource::with_application_version(Some("1.2.3"));

        assert_eq!(source.name(), APPLICATION_INFO_PROPERTY_SOURCE_NAME);
        assert_eq!(
            source.property(APPLICATION_VERSION_PROPERTY),
            Some("1.2.3".to_owned())
        );
        assert_eq!(source.property(APPLICATION_PID_PROPERTY), Some(pid()));
        assert!(source.is_immutable());
    }

    #[test]
    fn new_exposes_process_id_only() {
        let source = ApplicationInfoPropertySource::new();

        assert!(!source.contains_property(APPLICATION_VERSION_PROPERTY));
        assert_eq!(source.property(APPLICATION_PID_PROPERTY), Some(pid()));
    }

    #[test]
    fn omits_versions_without_text() {
        for application_version in [None, Some(""), Some("   ")] {
            let source =
                ApplicationInfoPropertySource::with_application_version(application_version);

            assert!(!source.contains_property(APPLICATION_VERSION_PROPERTY));
            assert_eq!(
                source.property_names(),
                vec![APPLICATION_PID_PROPERTY.to_owned()]
            );
        }
    }

    #[test]
    fn reads_version_from_main_class() {
        let source =
            ApplicationInfoPropertySource::from_main_class(Some("example::MainApplication"));

        assert_eq!(
            source.property(APPLICATION_VERSION_PROPERTY),
            Some(NextWebVersion::get_version().to_owned())
        );
    }

    #[test]
    fn ignores_main_class_without_text() {
        for main_class in [None, Some(""), Some("   ")] {
            let source = ApplicationInfoPropertySource::from_main_class(main_class);

            assert!(!source.contains_property(APPLICATION_VERSION_PROPERTY));
        }
    }

    #[test]
    fn move_to_end_reorders_last() {
        let mut environment = ApplicationEnvironment::default();
        environment
            .property_sources()
            .add_first(Box::new(ApplicationInfoPropertySource::new()));
        environment
            .property_sources()
            .add_last(Box::new(MapPropertySource::new(
                "other".to_owned(),
                IndexMap::new(),
            )));

        assert_eq!(
            environment
                .property_sources()
                .iter()
                .next()
                .map(|source| source.name()),
            Some(APPLICATION_INFO_PROPERTY_SOURCE_NAME)
        );

        ApplicationInfoPropertySource::move_to_end(&mut environment);

        assert_eq!(
            environment
                .property_sources()
                .iter()
                .last()
                .map(|source| source.name()),
            Some(APPLICATION_INFO_PROPERTY_SOURCE_NAME)
        );
    }

    #[test]
    fn move_to_end_ignores_missing_source() {
        let mut environment = ApplicationEnvironment::default();
        environment
            .property_sources()
            .add_first(Box::new(MapPropertySource::new(
                "other".to_owned(),
                IndexMap::new(),
            )));

        ApplicationInfoPropertySource::move_to_end(&mut environment);

        assert_eq!(environment.property_sources().len(), 1);
    }
}
