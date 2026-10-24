//! The error that is reported when a config data location that must exist
//! cannot be found.

use std::fmt;

use crate::context::config::config_data_location::ConfigDataLocation;

/// Error reported when a config data location that must exist cannot be
/// resolved to any resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDataLocationNotFoundError {
    location: ConfigDataLocation,
}

impl ConfigDataLocationNotFoundError {
    /// Creates a new error for the given location.
    ///
    /// # Arguments
    ///
    /// * `location` - The location that could not be resolved.
    pub fn new(location: ConfigDataLocation) -> Self {
        Self { location }
    }

    /// Returns the location that could not be resolved.
    pub fn location(&self) -> &ConfigDataLocation {
        &self.location
    }
}

impl fmt::Display for ConfigDataLocationNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Config data location '{}' does not exist", self.location)
    }
}

impl std::error::Error for ConfigDataLocationNotFoundError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_the_missing_location() {
        let error = ConfigDataLocationNotFoundError::new(ConfigDataLocation::of("file:./missing/"));

        assert_eq!(error.location(), &ConfigDataLocation::of("file:./missing/"));
        assert_eq!(
            error.to_string(),
            "Config data location 'file:./missing/' does not exist"
        );
    }
}
