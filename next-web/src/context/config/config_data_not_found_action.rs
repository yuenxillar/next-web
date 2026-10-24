//! The action to take when a config data location that must exist cannot be
//! found.

use tracing::debug;

use crate::context::config::config_data_location_not_found_error::ConfigDataLocationNotFoundError;

/// The property value that makes missing locations fail.
const FAIL: &str = "fail";

/// The property value that makes missing locations be ignored.
const IGNORE: &str = "ignore";

/// The action to take when a config data location that must exist cannot be
/// found.
///
/// The action is selected through the `next.config.on-not-found` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfigDataNotFoundAction {
    /// Report a [`ConfigDataLocationNotFoundError`] (the default).
    #[default]
    Fail,
    /// Log the failure and continue.
    Ignore,
}

impl ConfigDataNotFoundAction {
    /// Returns the action for the given property value, if it is a known
    /// action.
    ///
    /// The value is matched case insensitively, and unknown values are rejected
    /// so that callers keep their default.
    ///
    /// # Arguments
    ///
    /// * `value` - The value of the `next.config.on-not-found` property.
    pub fn of(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            FAIL => Some(Self::Fail),
            IGNORE => Some(Self::Ignore),
            _ => None,
        }
    }

    /// Handles a location that could not be found.
    ///
    /// # Arguments
    ///
    /// * `error` - The error raised while resolving the location.
    ///
    /// # Errors
    ///
    /// Returns the given error when the action is [`Fail`](Self::Fail).
    pub fn handle(
        &self,
        error: ConfigDataLocationNotFoundError,
    ) -> Result<(), ConfigDataLocationNotFoundError> {
        match self {
            Self::Fail => Err(error),
            Self::Ignore => {
                debug!(
                    "Config data location '{}' was not found, ignoring it (next.config.on-not-found=ignore)",
                    error.location()
                );
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::config::config_data_location::ConfigDataLocation;

    #[test]
    fn creates_actions_from_property_values() {
        assert_eq!(
            ConfigDataNotFoundAction::of("fail"),
            Some(ConfigDataNotFoundAction::Fail)
        );
        assert_eq!(
            ConfigDataNotFoundAction::of(" IGNORE "),
            Some(ConfigDataNotFoundAction::Ignore)
        );
        assert_eq!(ConfigDataNotFoundAction::of("unknown"), None);
    }

    #[test]
    fn defaults_to_failing() {
        assert_eq!(
            ConfigDataNotFoundAction::default(),
            ConfigDataNotFoundAction::Fail
        );
    }

    #[test]
    fn handle_fails_for_the_fail_action() {
        let error = ConfigDataLocationNotFoundError::new(ConfigDataLocation::of("file:./missing/"));

        let result = ConfigDataNotFoundAction::Fail.handle(error.clone());

        assert_eq!(result, Err(error));
    }

    #[test]
    fn handle_ignores_the_failure_for_the_ignore_action() {
        let error = ConfigDataLocationNotFoundError::new(ConfigDataLocation::of("file:./missing/"));

        assert!(ConfigDataNotFoundAction::Ignore.handle(error).is_ok());
    }
}
