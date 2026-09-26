//! The error returned when binding the properties of an environment fails.

use std::fmt;

/// The error returned when the properties of an environment cannot be bound to
/// the requested type.
///
/// The error carries the message of the failure, and the path of the field the
/// failure occurred at when the deserializer reports one. The path is the path
/// of the fields of the target type, so a field that is bound from a relaxed
/// property name is reported with the name of the field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindError {
    path: Option<String>,
    message: String,
}

impl BindError {
    /// Creates an error with the given message.
    ///
    /// # Arguments
    ///
    /// * `message` - The message that describes the failure.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            path: None,
            message: message.into(),
        }
    }

    /// Returns this error with the given field path.
    ///
    /// An empty path, or the path the deserializer reports for the root, does
    /// not describe a field and is therefore ignored.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the field the failure occurred at.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        let path = path.into();

        if !path.is_empty() && path != "." {
            self.path = Some(path);
        }

        self
    }

    /// Returns the message of this error.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the path of the field this error occurred at, if it is known.
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }
}

impl fmt::Display for BindError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.path {
            Some(path) => write!(formatter, "failed to bind `{path}`: {}", self.message),
            None => formatter.write_str(&self.message),
        }
    }
}

impl std::error::Error for BindError {}

impl serde::de::Error for BindError {
    fn custom<T>(message: T) -> Self
    where
        T: fmt::Display,
    {
        Self::new(message.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_the_message() {
        let error = BindError::new("invalid value");

        assert_eq!(error.message(), "invalid value");
        assert_eq!(error.path(), None);
        assert_eq!(error.to_string(), "invalid value");
    }

    #[test]
    fn reports_the_path_of_a_field() {
        let error = BindError::new("invalid value").with_path("cache_duration");

        assert_eq!(error.path(), Some("cache_duration"));
        assert_eq!(
            error.to_string(),
            "failed to bind `cache_duration`: invalid value"
        );
    }

    #[test]
    fn ignores_the_path_of_the_root() {
        assert_eq!(BindError::new("invalid value").with_path(".").path(), None);
        assert_eq!(BindError::new("invalid value").with_path("").path(), None);
    }

    #[test]
    fn creates_an_error_from_a_deserializer_message() {
        let error = <BindError as serde::de::Error>::custom("invalid type");

        assert_eq!(error.message(), "invalid type");
    }
}
