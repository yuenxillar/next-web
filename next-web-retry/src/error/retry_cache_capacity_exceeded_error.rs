//! The failure raised when a retry context cache is full.

use std::{
    error::Error,
    fmt,
    hash::{Hash, Hasher},
};

use next_web_core::any::AnyError;

use crate::error::retry_error::RetryError;

/// Raised when a retry context cache reaches its configured capacity.
///
/// A full cache usually means the keys used to identify failed items are unstable: the
/// same item is treated as new on every failure and keeps being added. Inspect the key
/// generator, or raise the capacity, when this surfaces.
#[derive(Clone, Debug)]
pub struct RetryCacheCapacityExceededError {
    /// The human readable message.
    pub msg: String,
    /// The failure that surfaced the limit, if any.
    pub cause: Option<Box<dyn AnyError>>,
}

impl RetryCacheCapacityExceededError {
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

impl PartialEq for RetryCacheCapacityExceededError {
    fn eq(&self, other: &Self) -> bool {
        self.msg == other.msg
    }
}

impl Eq for RetryCacheCapacityExceededError {}

impl Hash for RetryCacheCapacityExceededError {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.msg.hash(state);
    }
}

impl fmt::Display for RetryCacheCapacityExceededError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.cause {
            Some(cause) => write!(f, "{}: {cause}", self.msg),
            None => f.write_str(&self.msg),
        }
    }
}

impl Error for RetryCacheCapacityExceededError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause
            .as_ref()
            .map(|cause| cause.as_ref() as &(dyn Error + 'static))
    }
}

impl From<RetryCacheCapacityExceededError> for RetryError {
    fn from(error: RetryCacheCapacityExceededError) -> Self {
        match error.cause {
            Some(cause) => RetryError::RetryCacheCapacityExceededError(
                crate::error::retry_error::WithCauseError::with_cause(error.msg, cause),
            ),
            None => RetryError::RetryCacheCapacityExceededError(
                crate::error::retry_error::WithCauseError::new(error.msg),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RetryCacheCapacityExceededError;
    use crate::error::retry_error::{RetryError, RetryErrorKind};

    #[test]
    fn converts_into_the_root_error() {
        let converted: RetryError = RetryCacheCapacityExceededError::new("cache is full").into();

        assert_eq!(converted.kind(), RetryErrorKind::RetryCacheCapacityExceeded);
    }
}
