//! The error that is reported when an invalid property is found in the config
//! data.

use std::fmt;

use crate::context::config::config_data_environment_contributor::ConfigDataEnvironmentContributor;
use crate::context::config::config_data_location::ConfigDataLocation;
use crate::context::config::config_data_profiles::ACTIVE_PROFILE_PROPERTY;

/// The property that is replaced by `next.profiles.active` and
/// `next.profiles.default`.
const LEGACY_PROFILES_PROPERTY: &str = "next.profiles";

/// The reason a property is invalid when it is used in a profile specific
/// document.
const PROFILE_SPECIFIC_REASON: &str =
    "the property must not be used in a profile specific document";

/// The reason the legacy profiles property is invalid.
const LEGACY_REASON: &str =
    "the property must be replaced with 'next.profiles.active' or 'next.profiles.default'";

/// Error reported when a property that is not allowed in the config data is
/// found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidConfigDataPropertyError {
    property_name: String,
    location: Option<ConfigDataLocation>,
    reason: &'static str,
}

impl InvalidConfigDataPropertyError {
    /// Checks the given contributor for properties that are not allowed.
    ///
    /// # Arguments
    ///
    /// * `contributor` - The contributor to check.
    ///
    /// # Errors
    ///
    /// Returns the first invalid property that is found.
    pub(crate) fn check(contributor: &ConfigDataEnvironmentContributor) -> Result<(), Self> {
        let Some(property_source) = contributor.property_source() else {
            return Ok(());
        };

        if property_source.contains_property(LEGACY_PROFILES_PROPERTY) {
            return Err(Self::new(
                LEGACY_PROFILES_PROPERTY,
                contributor.location(),
                LEGACY_REASON,
            ));
        }

        let profile_specific = contributor.is_ignore_profiles()
            && property_source.contains_property(ACTIVE_PROFILE_PROPERTY);
        if profile_specific {
            return Err(Self::new(
                ACTIVE_PROFILE_PROPERTY,
                contributor.location(),
                PROFILE_SPECIFIC_REASON,
            ));
        }

        Ok(())
    }

    /// Creates a new error.
    fn new(
        property_name: &str,
        location: Option<&ConfigDataLocation>,
        reason: &'static str,
    ) -> Self {
        Self {
            property_name: property_name.to_owned(),
            location: location.cloned(),
            reason,
        }
    }

    /// Returns the name of the invalid property.
    pub fn property_name(&self) -> &str {
        &self.property_name
    }

    /// Returns the location the invalid property was imported from, if any.
    pub fn location(&self) -> Option<&ConfigDataLocation> {
        self.location.as_ref()
    }
}

impl fmt::Display for InvalidConfigDataPropertyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Property '{}' imported from '{}' is invalid: {}",
            self.property_name,
            self.location
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "the environment".to_owned()),
            self.reason
        )
    }
}

impl std::error::Error for InvalidConfigDataPropertyError {}

#[cfg(test)]
mod tests {
    use next_web_core::util::indexmap::IndexMap;

    use crate::context::config::config_data_loader::ConfigData;
    use crate::env::MapPropertySource;

    use super::*;

    /// Creates a contributor for the given properties.
    fn contributor(
        properties: &[(&str, &str)],
        profile_specific: bool,
    ) -> ConfigDataEnvironmentContributor {
        let properties: IndexMap<String, String> = properties
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();

        ConfigDataEnvironmentContributor::of_config_data(
            ConfigDataLocation::of("/application.properties"),
            ConfigData::new(Box::new(MapPropertySource::new(
                "application.properties".to_owned(),
                properties,
            ))),
            profile_specific,
        )
    }

    #[test]
    fn accepts_valid_properties() {
        let contributor = contributor(&[("next.application.name", "demo")], false);

        assert!(InvalidConfigDataPropertyError::check(&contributor).is_ok());
    }

    #[test]
    fn rejects_the_legacy_profiles_property() {
        let contributor = contributor(&[("next.profiles", "dev")], false);

        let error = InvalidConfigDataPropertyError::check(&contributor).unwrap_err();

        assert_eq!(error.property_name(), "next.profiles");
        assert_eq!(
            error.location(),
            Some(&ConfigDataLocation::of("/application.properties"))
        );
        assert!(error.to_string().contains("next.profiles"));
    }

    #[test]
    fn rejects_active_profiles_in_profile_specific_documents() {
        let contributor = contributor(&[("next.profiles.active", "dev")], true);

        let error = InvalidConfigDataPropertyError::check(&contributor).unwrap_err();

        assert_eq!(error.property_name(), "next.profiles.active");
    }
}
