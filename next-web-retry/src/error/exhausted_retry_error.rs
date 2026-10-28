//! The failure raised when every attempt was used up.

use std::{
    error::Error,
    fmt,
    hash::{Hash, Hasher},
};

use next_web_core::any::AnyError;

use crate::error::retry_error::RetryError;

/// Raised when every attempt was used up and no recovery value was produced.
///
/// The template raises it for stateful retries that neither recovered nor asked for the
/// last error to be rethrown. The original failure is kept as the cause.
#[derive(Clone, Debug)]
pub struct ExhaustedRetryError {
    /// The human readable message.
    pub msg: String,
    /// The failure that exhausted the retry, if known.
    pub cause: Option<Box<dyn AnyError>>,
}

impl ExhaustedRetryError {
    /// Creates the error from a message.
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into(),
            cause: None,
        }
    }

    /// Creates the error from a message and the failure that exhausted the retry.
    pub fn with_cause(msg: impl Into<String>, cause: Box<dyn AnyError>) -> Self {
        Self {
            msg: msg.into(),
            cause: Some(cause),
        }
    }
}

impl PartialEq for ExhaustedRetryError {
    fn eq(&self, other: &Self) -> bool {
        self.msg == other.msg
    }
}

impl Eq for ExhaustedRetryError {}

impl Hash for ExhaustedRetryError {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.msg.hash(state);
    }
}

impl fmt::Display for ExhaustedRetryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.cause {
            Some(cause) => write!(f, "{}: {cause}", self.msg),
            None => f.write_str(&self.msg),
        }
    }
}

impl Error for ExhaustedRetryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause
            .as_ref()
            .map(|cause| cause.as_ref() as &(dyn Error + 'static))
    }
}

impl From<ExhaustedRetryError> for RetryError {
    fn from(error: ExhaustedRetryError) -> Self {
        match error.cause {
            Some(cause) => RetryError::ExhaustedRetryError(
                crate::error::retry_error::WithCauseError::with_cause(error.msg, cause),
            ),
            None => RetryError::ExhaustedRetryError(
                crate::error::retry_error::WithCauseError::new(error.msg),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ExhaustedRetryError;
    use crate::error::retry_error::RetryError;

    #[test]
    fn converts_into_the_root_error_and_keeps_the_cause() {
        let error = ExhaustedRetryError::with_cause(
            "gave up",
            Box::new(RetryError::custom("connection reset")),
        );
        let converted: RetryError = error.into();

        assert!(converted.is_exhausted());
        assert_eq!(converted.to_string(), "gave up: connection reset");
    }
}
