//! The failure raised when waiting between attempts is interrupted.

use std::{
    error::Error,
    fmt,
    hash::{Hash, Hasher},
};

use next_web_core::any::AnyError;

use crate::error::retry_error::RetryError;

/// Raised when waiting between two attempts is interrupted.
///
/// A back-off policy returns it when its sleeper is cancelled or fails, and the template
/// aborts the whole retry instead of trying again: the wait was prevented by another
/// actor, so continuing would defeat the purpose of the back off.
#[derive(Clone, Debug)]
pub struct BackOffInterruptedError {
    /// The human readable message.
    pub msg: String,
    /// The failure that interrupted the wait, if any.
    pub cause: Option<Box<dyn AnyError>>,
}

impl BackOffInterruptedError {
    /// Creates the error from a message.
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into(),
            cause: None,
        }
    }

    /// Creates the error from a message and the failure that interrupted the wait.
    pub fn with_cause(msg: impl Into<String>, cause: Box<dyn AnyError>) -> Self {
        Self {
            msg: msg.into(),
            cause: Some(cause),
        }
    }
}

impl PartialEq for BackOffInterruptedError {
    fn eq(&self, other: &Self) -> bool {
        self.msg == other.msg
    }
}

impl Eq for BackOffInterruptedError {}

impl Hash for BackOffInterruptedError {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.msg.hash(state);
    }
}

impl fmt::Display for BackOffInterruptedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.cause {
            Some(cause) => write!(f, "{}: {cause}", self.msg),
            None => f.write_str(&self.msg),
        }
    }
}

impl Error for BackOffInterruptedError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause
            .as_ref()
            .map(|cause| cause.as_ref() as &(dyn Error + 'static))
    }
}

impl From<BackOffInterruptedError> for RetryError {
    fn from(error: BackOffInterruptedError) -> Self {
        match error.cause {
            Some(cause) => RetryError::BackOffInterruptedError(
                crate::error::retry_error::WithCauseError::with_cause(error.msg, cause),
            ),
            None => RetryError::BackOffInterruptedError(
                crate::error::retry_error::WithCauseError::new(error.msg),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BackOffInterruptedError;
    use crate::error::retry_error::RetryError;

    #[test]
    fn converts_into_the_root_error() {
        let converted: RetryError = BackOffInterruptedError::new("interrupted").into();

        assert!(converted.is_back_off_interrupted());
    }
}
