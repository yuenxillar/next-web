//! The contributors of the config data environment.

use crate::context::config::config_data_activation_context::ConfigDataActivationContext;
use crate::context::config::config_data_environment_contributor::ConfigDataEnvironmentContributor;
use crate::context::config::config_data_importer::ConfigDataImporter;
use crate::context::config::config_data_location_not_found_error::ConfigDataLocationNotFoundError;

/// The maximum number of nested imports that are followed.
///
/// The limit guards against locations that import themselves, directly or
/// through another document.
pub(crate) const MAX_IMPORT_DEPTH: usize = 4;

/// The contributors that make up the config data environment.
///
/// Contributors are searched in precedence order, and the imports they declare
/// are expanded in place, so that the property sources of an imported document
/// are added ahead of the document that imports it.
#[derive(Debug, Default)]
pub(crate) struct ConfigDataEnvironmentContributors {
    contributors: Vec<ConfigDataEnvironmentContributor>,
}

impl ConfigDataEnvironmentContributors {
    /// Creates a new collection for the given contributors.
    ///
    /// # Arguments
    ///
    /// * `contributors` - The contributors, in descending precedence order.
    pub(crate) fn new(contributors: Vec<ConfigDataEnvironmentContributor>) -> Self {
        Self { contributors }
    }

    /// Returns a flattened view of the contributors, in descending precedence
    /// order.
    pub(crate) fn flattened(&self) -> Vec<&ConfigDataEnvironmentContributor> {
        let mut result = Vec::new();
        for contributor in &self.contributors {
            contributor.flatten(&mut result);
        }
        result
    }

    /// Consumes the collection and returns its contributors, flattened in
    /// descending precedence order.
    pub(crate) fn into_flattened(self) -> Vec<ConfigDataEnvironmentContributor> {
        let mut result = Vec::new();
        for contributor in self.contributors {
            contributor.into_flattened(&mut result);
        }
        result
    }

    /// Returns the value of the given profile property, taken from the first
    /// contributor that is allowed to influence the profile activation.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the profile property.
    pub(crate) fn get_profile_property(&self, name: &str) -> Option<String> {
        self.flattened()
            .into_iter()
            .filter(|contributor| !contributor.is_ignore_profiles())
            .find_map(|contributor| contributor.property_source()?.property(name))
    }

    /// Returns the values of the given profile property from every contributor
    /// that is allowed to influence the profile activation.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the profile property.
    pub(crate) fn get_profile_property_values(&self, name: &str) -> Vec<String> {
        self.flattened()
            .into_iter()
            .filter(|contributor| !contributor.is_ignore_profiles())
            .filter_map(|contributor| contributor.property_source()?.property(name))
            .collect()
    }

    /// Expands every pending import of the contributors.
    ///
    /// The method can be called once per processing phase: locations that were
    /// already loaded are not loaded again, so calling it a second time with
    /// the profiles in place only adds the profile specific documents of the
    /// locations.
    ///
    /// # Arguments
    ///
    /// * `importer` - The importer used to resolve and load the locations.
    /// * `activation_context` - The context holding the active profiles.
    ///
    /// # Errors
    ///
    /// Returns an error when a location that must exist cannot be found.
    pub(crate) fn expand_imports(
        &mut self,
        importer: &mut ConfigDataImporter<'_>,
        activation_context: &ConfigDataActivationContext,
    ) -> Result<(), ConfigDataLocationNotFoundError> {
        for contributor in &mut self.contributors {
            expand_contributor(contributor, importer, activation_context, 0)?;
        }
        Ok(())
    }
}

/// Expands the imports of a single contributor, and of the contributors it
/// loaded.
fn expand_contributor(
    contributor: &mut ConfigDataEnvironmentContributor,
    importer: &mut ConfigDataImporter<'_>,
    activation_context: &ConfigDataActivationContext,
    depth: usize,
) -> Result<(), ConfigDataLocationNotFoundError> {
    if depth >= MAX_IMPORT_DEPTH {
        return Ok(());
    }

    if contributor.has_imports() {
        let imports = contributor.imports().to_vec();
        let mut children = Vec::new();
        // The last location that is declared wins, so the imports are resolved
        // from the last one to the first one.
        for location in imports.iter().rev() {
            children.extend(importer.resolve_and_load(location, activation_context)?);
        }
        contributor.prepend_children(children);
    }

    for child in contributor.children_mut() {
        expand_contributor(child, importer, activation_context, depth + 1)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use next_web_core::util::indexmap::IndexMap;

    use crate::context::config::config_data_loader::ConfigData;
    use crate::context::config::config_data_location::ConfigDataLocation;
    use crate::env::MapPropertySource;

    use super::*;

    /// Creates a map backed property source.
    fn property_source(
        name: &str,
        properties: &[(&str, &str)],
    ) -> next_web_core::env::BoxedPropertySource {
        let properties: IndexMap<String, String> = properties
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        Box::new(MapPropertySource::new(name.to_owned(), properties))
    }

    /// Creates a contributor holding the given properties.
    fn contributor(
        name: &str,
        properties: &[(&str, &str)],
        profile_specific: bool,
    ) -> ConfigDataEnvironmentContributor {
        ConfigDataEnvironmentContributor::of_config_data(
            ConfigDataLocation::of("/application.properties"),
            ConfigData::new(property_source(name, properties)),
            profile_specific,
        )
    }

    #[test]
    fn ignores_profile_specific_documents_for_profile_properties() {
        let contributors = ConfigDataEnvironmentContributors::new(vec![
            contributor(
                "profile-specific",
                &[
                    ("next.config.activate.on-profile", "dev"),
                    ("next.profiles.include", "cloud"),
                ],
                false,
            ),
            contributor("regular", &[("next.profiles.include", "local")], false),
        ]);

        assert_eq!(
            contributors.get_profile_property_values("next.profiles.include"),
            vec!["local".to_owned()]
        );
        assert_eq!(
            contributors.get_profile_property("next.profiles.include"),
            Some("local".to_owned())
        );
    }

    #[test]
    fn flattens_contributors_in_precedence_order() {
        let mut parent = contributor("parent", &[], false);
        parent.prepend_children(vec![contributor("child", &[], false)]);
        let contributors = ConfigDataEnvironmentContributors::new(vec![parent]);

        let names: Vec<_> = contributors
            .into_flattened()
            .iter()
            .map(|contributor| contributor.property_source().unwrap().name().to_owned())
            .collect();

        assert_eq!(names, vec!["child".to_owned(), "parent".to_owned()]);
    }
}
