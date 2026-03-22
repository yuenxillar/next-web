use std::io;

use next_web_core::error::BoxError;

/// General exception indicating a problem in ensemble.
#[derive(Debug)]
pub struct StateMachineEnsembleError {
    /// Error message
    message: String,
    /// Underlying cause, if any
    cause: Option<BoxError>,
}

impl StateMachineEnsembleError {
    /// Creates a new state machine ensemble exception from an I/O error.
    ///
    /// # Arguments
    /// * `e` - The I/O error
    ///
    /// # Returns
    /// A new `StateMachineEnsembleError` instance
    pub fn from_io(e: io::Error) -> Self {
        Self {
            message: e.to_string(),
            cause: Some(Box::new(e)),
        }
    }

    /// Creates a new state machine ensemble exception with a message and underlying exception.
    ///
    /// # Arguments
    /// * `message` - The error message
    /// * `e` - The underlying exception
    ///
    /// # Returns
    /// A new `StateMachineEnsembleError` instance
    pub fn with_exception(
        message: impl Into<String>,
        e: Box<dyn std::error::Error + Send + Sync>,
    ) -> Self {
        Self {
            message: message.into(),
            cause: Some(e),
        }
    }

    /// Creates a new state machine ensemble exception with a message and cause.
    ///
    /// # Arguments
    /// * `message` - The error message
    /// * `cause` - The underlying cause
    ///
    /// # Returns
    /// A new `StateMachineEnsembleError` instance
    pub fn with_cause(
        message: impl Into<String>,
        cause: Box<dyn std::error::Error + Send + Sync>,
    ) -> Self {
        Self {
            message: message.into(),
            cause: Some(cause),
        }
    }

    /// Creates a new state machine ensemble exception with a message.
    ///
    /// # Arguments
    /// * `message` - The error message
    ///
    /// # Returns
    /// A new `StateMachineEnsembleError` instance
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            cause: None,
        }
    }

    /// Gets the error message.
    ///
    /// # Returns
    /// The error message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Gets the underlying cause, if any.
    ///
    /// # Returns
    /// The underlying cause, if present
    pub fn cause(&self) -> Option<&(dyn std::error::Error + Send + Sync)> {
        self.cause.as_deref()
    }
}

impl std::fmt::Display for StateMachineEnsembleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StateMachineEnsembleError: {}", self.message)?;
        if let Some(cause) = &self.cause {
            write!(f, " (caused by: {})", cause)?;
        }
        Ok(())
    }
}

impl std::error::Error for StateMachineEnsembleError {}

impl From<io::Error> for StateMachineEnsembleError {
    fn from(e: io::Error) -> Self {
        Self::from_io(e)
    }
}

impl From<String> for StateMachineEnsembleError {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl From<&str> for StateMachineEnsembleError {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}
