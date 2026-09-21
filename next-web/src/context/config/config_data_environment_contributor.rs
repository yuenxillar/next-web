//! A single contributor of config data.

use std::fmt;

use next_web_core::env::{BoxedPropertySource, PropertySource, PropertySourceValue};

use crate::context::config::config_data_activation_context::ConfigDataActivationContext;
use crate::context::config::config_data_loader::ConfigData;
use crate::context::config::config_data_location::ConfigDataLocation;

/// A contributor of config data.
///
/// A contributor is a node in the tree that holds the documents that are loaded
/// for the locations of the environment. It holds the locations it imports and,
/// once those locations have been processed, the contributors that were loaded
/// from them.
///
/// Children are iterated before their parent, so an imported document takes
/// precedence over the document that imports it.
pub struct ConfigDataEnvironmentContributor {
    location: Option<ConfigDataLocation>,
    property_source: Option<BoxedPropertySource>,
    imports: Vec<ConfigDataLocation>,
    children: Vec<ConfigDataEnvironmentContributor>,
    activate_profile: Option<String>,
    ignore_profiles: bool,
}

impl ConfigDataEnvironmentContributor {
    /// Creates a contributor for a set of locations that have to be imported.
    ///
    /// # Arguments
    ///
    /// * `locations` - The locations to import.
    pub fn initial_imports(locations: Vec<ConfigDataLocation>) -> Self {
        Self {
            location: None,
            property_source: None,
            imports: locations,
            children: Vec::new(),
            activate_profile: None,
            ignore_profiles: false,
        }
    }

    /// Creates a contributor for a document that has been loaded.
    ///
    /// # Arguments
    ///
    /// * `location` - The location the document was loaded from.
    /// * `config` - The loaded document.
    /// * `profile_specific` - Whether the document was resolved for a profile,
    ///   for example because it is named after one.
    pub fn of_config_data(
        location: ConfigDataLocation,
        config: ConfigData,
        profile_specific: bool,
    ) -> Self {
        let imports = config.imports();
        let activate_profile = config.activate_profile().map(ToOwned::to_owned);
        let ignore_profiles = profile_specific || activate_profile.is_some();

        Self {
            location: Some(location),
            property_source: Some(config.into_property_source()),
            imports,
            children: Vec::new(),
            activate_profile,
            ignore_profiles,
        }
    }

    /// Returns the location this contributor was loaded from, if any.
    pub fn location(&self) -> Option<&ConfigDataLocation> {
        self.location.as_ref()
    }

    /// Returns the property source of this contributor, if any.
    pub fn property_source(&self) -> Option<&dyn PropertySource<PropertySourceValue>> {
        self.property_source.as_deref()
    }

    /// Returns the property source of this contributor, if any.
    pub fn into_property_source(mut self) -> Option<BoxedPropertySource> {
        self.property_source.take()
    }

    /// Returns the locations imported by this contributor.
    pub fn imports(&self) -> &[ConfigDataLocation] {
        &self.imports
    }

    /// Returns whether this contributor imports any location.
    pub fn has_imports(&self) -> bool {
        !self.imports.is_empty()
    }

    /// Returns the locations imported by this contributor that must exist.
    pub fn mandatory_imports(&self) -> Vec<ConfigDataLocation> {
        self.imports
            .iter()
            .filter(|location| !location.is_optional())
            .cloned()
            .collect()
    }

    /// Returns whether this contributor may take part in the deduction of the
    /// active profiles.
    ///
    /// Documents that are specific to a profile do not take part, since they
    /// are only loaded because a profile is already active.
    pub fn is_ignore_profiles(&self) -> bool {
        self.ignore_profiles
    }

    /// Returns whether this contributor is active for the given activation
    /// context.
    ///
    /// A contributor without an activation profile expression is always active.
    ///
    /// # Arguments
    ///
    /// * `activation_context` - The context holding the active profiles.
    pub fn is_active(&self, activation_context: &ConfigDataActivationContext) -> bool {
        match &self.activate_profile {
            Some(expression) => activation_context.accepts(&[expression.as_str()]),
            None => true,
        }
    }

    /// Returns the loaded contributors of this contributor.
    pub(crate) fn children_mut(&mut self) -> &mut Vec<ConfigDataEnvironmentContributor> {
        &mut self.children
    }

    /// Adds the given contributors ahead of the already loaded ones, so that
    /// they take precedence.
    ///
    /// # Arguments
    ///
    /// * `children` - The contributors to add, in descending precedence order.
    pub(crate) fn prepend_children(&mut self, mut children: Vec<ConfigDataEnvironmentContributor>) {
        if children.is_empty() {
            return;
        }
        children.append(&mut self.children);
        self.children = children;
    }

    /// Flattens this contributor and its children into the given list, in
    /// descending precedence order.
    ///
    /// # Arguments
    ///
    /// * `result` - The list to add this contributor to.
    pub(crate) fn flatten<'a>(&'a self, result: &mut Vec<&'a Self>) {
        for child in &self.children {
            child.flatten(result);
        }
        result.push(self);
    }

    /// Flattens this contributor and its children into the given list, in
    /// descending precedence order.
    ///
    /// # Arguments
    ///
    /// * `result` - The list to add this contributor to.
    pub(crate) fn into_flattened(mut self, result: &mut Vec<Self>) {
        let children = std::mem::take(&mut self.children);
        for child in children {
            child.into_flattened(result);
        }
        result.push(self);
    }
}

impl fmt::Debug for ConfigDataEnvironmentContributor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConfigDataEnvironmentContributor")
            .field("location", &self.location.as_ref().map(ToString::to_string))
            .field(
                "property_source",
                &self.property_source.as_ref().map(|source| source.name()),
            )
            .field("imports", &self.imports)
            .field("children", &self.children.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use next_web_core::util::indexmap::IndexMap;

    use crate::env::MapPropertySource;

    use super::*;

    /// Creates a map backed property source.
    fn property_source(name: &str, properties: &[(&str, &str)]) -> BoxedPropertySource {
        let properties: IndexMap<String, String> = properties
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        Box::new(MapPropertySource::new(name.to_owned(), properties))
    }

    /// Creates the config data of a document.
    fn config_data(name: &str, properties: &[(&str, &str)]) -> ConfigData {
        ConfigData::new(property_source(name, properties))
    }

    /// Creates a contributor for the given properties.
    fn contributor(
        properties: &[(&str, &str)],
        profile_specific: bool,
    ) -> ConfigDataEnvironmentContributor {
        ConfigDataEnvironmentContributor::of_config_data(
            ConfigDataLocation::of("/application.properties"),
            config_data("application.properties", properties),
            profile_specific,
        )
    }

    #[test]
    fn exposes_the_properties_of_a_document() {
        let contributor = contributor(&[("next.application.name", "demo")], false);

        assert_eq!(
            contributor
                .property_source()
                .and_then(|source| source.property("next.application.name")),
            Some("demo".to_owned())
        );
        assert_eq!(
            contributor.location(),
            Some(&ConfigDataLocation::of("/application.properties"))
        );
        assert!(!contributor.has_imports());
        assert!(contributor.is_active(&ConfigDataActivationContext::new(None)));
    }

    #[test]
    fn reports_the_mandatory_imports_of_a_document() {
        let contributor = contributor(
            &[(
                "next.config.import",
                "/one.properties,optional:/two.properties",
            )],
            false,
        );

        assert_eq!(
            contributor.mandatory_imports(),
            vec![ConfigDataLocation::of("/one.properties")]
        );
        assert!(!contributor.is_ignore_profiles());
    }

    #[test]
    fn marks_documents_that_must_not_activate_profiles() {
        let contributor = contributor(&[("next.config.activate.on-profile", "dev")], false);

        assert!(contributor.is_ignore_profiles());
        assert!(!contributor.is_active(&ConfigDataActivationContext::new(None)));
    }

    #[test]
    fn flattens_children_before_their_parent() {
        let mut parent = contributor(&[], false);
        parent.prepend_children(vec![
            ConfigDataEnvironmentContributor::of_config_data(
                ConfigDataLocation::of("/first"),
                config_data("first", &[]),
                false,
            ),
            ConfigDataEnvironmentContributor::of_config_data(
                ConfigDataLocation::of("/second"),
                config_data("second", &[]),
                false,
            ),
        ]);

        let mut result = Vec::new();
        parent.flatten(&mut result);
        let names: Vec<_> = result
            .iter()
            .map(|contributor| contributor.property_source().unwrap().name())
            .collect();

        assert_eq!(names, vec!["first", "second", "application.properties"]);
    }

    #[test]
    fn prepending_keeps_the_new_children_first() {
        let mut parent = contributor(&[], false);
        parent.prepend_children(vec![ConfigDataEnvironmentContributor::of_config_data(
            ConfigDataLocation::of("/first"),
            config_data("first", &[]),
            false,
        )]);
        parent.prepend_children(vec![ConfigDataEnvironmentContributor::of_config_data(
            ConfigDataLocation::of("/second"),
            config_data("second", &[]),
            true,
        )]);

        let mut result = Vec::new();
        parent.flatten(&mut result);
        let names: Vec<_> = result
            .iter()
            .map(|contributor| contributor.property_source().unwrap().name())
            .collect();

        assert_eq!(names, vec!["second", "first", "application.properties"]);
    }
}
