//! Error types used by the placeholder utilities.

use std::fmt;

/// A boxed error type, matching the convention used across the workspace.
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Errorthrown when a placeholder cannot be resolved.
///
/// Corresponds to the `PlaceholderResolutionError`. While the parsing
/// stack unwinds, the value that contained the offending placeholder can be
/// attached with [`PlaceholderResolutionError::with_value`]; it is then
/// included in the rendered message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceholderResolutionError {
    message: String,
    placeholder: String,
    original_value: Option<String>,
    value: Option<String>,
}

impl PlaceholderResolutionError {
    /// Creates a new Error.
    pub fn new(
        message: impl Into<String>,
        placeholder: impl Into<String>,
        original_value: Option<String>,
    ) -> Self {
        Self {
            message: message.into(),
            placeholder: placeholder.into(),
            original_value,
            value: None,
        }
    }

    /// Creates a new Errorfrom a plain message, without any placeholder
    /// metadata.
    pub fn with_message(message: impl Into<String>) -> Self {
        Self::new(message, String::new(), None)
    }

    /// Returns the placeholder that could not be resolved.
    pub fn placeholder(&self) -> &str {
        &self.placeholder
    }

    /// Returns the original placeholder text, if it differs from the key.
    pub fn original_value(&self) -> Option<&str> {
        self.original_value.as_deref()
    }

    /// Returns the value that contained the offending placeholder, if known.
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    /// Returns the base message, without the optional `value` suffix.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Attaches the value that contained the offending placeholder.
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }
}

impl fmt::Display for PlaceholderResolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.value {
            Some(value) => write!(f, "{} in value \"{}\"", self.message, value),
            None => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for PlaceholderResolutionError {}
