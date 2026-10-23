//! The failure raised when a listener vetoes a retry.

use std::{
    error::Error,
    fmt,
    hash::{Hash, Hasher},
};

use next_web_core::any::AnyError;

use crate::error::retry_error::RetryError;

/// Raised when a listener vetoes a retry before the first attempt is made.
///
/// The template raises it when any registered listener returns `false` from its `open`
/// hook, and when the retry could not register a failure with the policy.
#[derive(Clone, Debug)]
pub struct TerminatedRetryError {
    /// The human readable message.
    pub msg: String,
    /// The failure that terminated the retry, if any.
    pub cause: Option<Box<dyn AnyError>>,
}

impl TerminatedRetryError {
    /// Creates the error from a message.
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into(),
            cause: None,
        }
    }

    /// Creates the error from a message and a cause.
    pub fn with_cause(msg: impl Into<String>, cause: Box<dyn AnyError>) -> Self {
        Self {
            msg: msg.into(),
            cause: Some(cause),
        }
    }
}

impl PartialEq for TerminatedRetryError {
    fn eq(&self, other: &Self) -> bool {
        self.msg == other.msg
    }
}

impl Eq for TerminatedRetryError {}

impl Hash for TerminatedRetryError {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.msg.hash(state);
    }
}

impl fmt::Display for TerminatedRetryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.cause {
            Some(cause) => write!(f, "{}: {cause}", self.msg),
            None => f.write_str(&self.msg),
        }
    }
}

impl Error for TerminatedRetryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause
            .as_ref()
            .map(|cause| cause.as_ref() as &(dyn Error + 'static))
    }
}

impl From<TerminatedRetryError> for RetryError {
    fn from(error: TerminatedRetryError) -> Self {
        match error.cause {
            Some(cause) => RetryError::TerminatedRetryError(
                crate::error::retry_error::WithCauseError::with_cause(error.msg, cause),
            ),
            None => RetryError::TerminatedRetryError(
                crate::error::retry_error::WithCauseError::new(error.msg),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TerminatedRetryError;
    use crate::error::retry_error::RetryError;

    #[test]
    fn converts_into_the_root_error() {
        let converted: RetryError = TerminatedRetryError::new("vetoed").into();

        assert!(converted.is_terminated());
        assert_eq!(converted.to_string(), "vetoed");
    }
}
