//! The single error type of every retry operation.

use std::{
    error::Error,
    fmt,
    hash::{Hash, Hasher},
};

use next_web_core::{any::AnyError, error::BoxError};

/// The concrete failure a [`RetryError`] represents.
///
/// The kind is the comparison point a caller uses when it needs to know *what* failed
/// without matching every variant or inspecting the message. It mirrors the exception
/// hierarchy of the original API: everything that is not one of the specialised failures
/// is reported as [`RetryErrorKind::Retry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RetryErrorKind {
    /// A generic retry failure, raised by the template itself.
    Retry,
    /// All attempts were used up and no recovery result was produced.
    ExhaustedRetry,
    /// A listener vetoed the retry before the first attempt.
    TerminatedRetry,
    /// Waiting between attempts was interrupted.
    BackOffInterrupted,
    /// The retry context cache reached its configured capacity.
    RetryCacheCapacityExceeded,
    /// The default exhausted-retry failure produced by the template.
    Default,
    /// A message-only error supplied by user code.
    Custom,
}

impl RetryErrorKind {
    /// Returns the stable, human readable name of this kind.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            RetryErrorKind::Retry => "retry",
            RetryErrorKind::ExhaustedRetry => "exhausted-retry",
            RetryErrorKind::TerminatedRetry => "terminated-retry",
            RetryErrorKind::BackOffInterrupted => "back-off-interrupted",
            RetryErrorKind::RetryCacheCapacityExceeded => "retry-cache-capacity-exceeded",
            RetryErrorKind::Default => "default",
            RetryErrorKind::Custom => "custom",
        }
    }
}

impl fmt::Display for RetryErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An error message paired with an optional cause.
///
/// The specialised retry failures share this shape: a human readable message and the
/// error that triggered it, which is what makes their cause chain inspectable through
/// [`Error::source`].
#[derive(Clone)]
pub struct WithCauseError {
    /// The human readable message.
    pub msg: String,
    /// The error that caused this one, if any.
    pub cause: Option<Box<dyn AnyError>>,
}

impl WithCauseError {
    /// Creates an error with a message and no cause.
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into(),
            cause: None,
        }
    }

    /// Creates an error with a message and a cause.
    pub fn with_cause(msg: impl Into<String>, cause: Box<dyn AnyError>) -> Self {
        Self {
            msg: msg.into(),
            cause: Some(cause),
        }
    }
}

impl Default for WithCauseError {
    fn default() -> Self {
        Self::new("retry error")
    }
}

impl PartialEq for WithCauseError {
    fn eq(&self, other: &Self) -> bool {
        self.msg == other.msg
    }
}

impl Eq for WithCauseError {}

impl Hash for WithCauseError {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.msg.hash(state);
    }
}

impl fmt::Debug for WithCauseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WithCauseError")
            .field("msg", &self.msg)
            .field("cause", &self.cause.as_ref().map(ToString::to_string))
            .finish()
    }
}

impl fmt::Display for WithCauseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.cause {
            Some(cause) => write!(f, "{}: {cause}", self.msg),
            None => f.write_str(&self.msg),
        }
    }
}

impl Error for WithCauseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause
            .as_ref()
            .map(|cause| cause.as_ref() as &(dyn Error + 'static))
    }
}

/// The error type of every retry operation.
///
/// A retry operation can fail for many reasons, and callers need to tell them apart
/// without losing the original error. `RetryError` keeps both: [`RetryError::kind`]
/// answers *what* failed, while the wrapped [`AnyError`] keeps the original error
/// intact and downcastable.
///
/// # Equality
///
/// Equality and hashing use the [`RetryErrorKind`] and the rendered message, never the
/// cause. Two failures that report the same kind and message compare equal even when
/// their causes differ. This is what makes an error classifier usable: a policy is
/// configured with a representative error and matches concrete failures by kind and
/// message.
#[derive(Clone, Debug)]
pub enum RetryError {
    /// A message-only failure supplied by user code.
    Custom(String),
    /// An arbitrary user error, stored type-erased.
    Any(Box<dyn AnyError>),
    /// All attempts were used up.
    ExhaustedRetryError(WithCauseError),
    /// A listener vetoed the retry before the first attempt.
    TerminatedRetryError(WithCauseError),
    /// Waiting between two attempts was interrupted.
    BackOffInterruptedError(WithCauseError),
    /// The retry context cache reached its configured capacity.
    RetryCacheCapacityExceededError(WithCauseError),
    /// The generic failure the template produces when nothing more specific applies.
    Default(WithCauseError),
}

impl RetryError {
    /// Creates a message-only error.
    pub fn custom(msg: impl Into<String>) -> Self {
        RetryError::Custom(msg.into())
    }

    /// Creates a generic retry failure.
    pub fn retry(msg: impl Into<String>) -> Self {
        RetryError::Default(WithCauseError::new(msg))
    }

    /// Creates a generic retry failure with a cause.
    pub fn retry_with_cause(msg: impl Into<String>, cause: Box<dyn AnyError>) -> Self {
        RetryError::Default(WithCauseError::with_cause(msg, cause))
    }

    /// Creates an exhausted-retry failure.
    pub fn exhausted(msg: impl Into<String>) -> Self {
        RetryError::ExhaustedRetryError(WithCauseError::new(msg))
    }

    /// Creates an exhausted-retry failure with a cause.
    pub fn exhausted_with_cause(msg: impl Into<String>, cause: Box<dyn AnyError>) -> Self {
        RetryError::ExhaustedRetryError(WithCauseError::with_cause(msg, cause))
    }

    /// Creates a terminated-retry failure.
    pub fn terminated(msg: impl Into<String>) -> Self {
        RetryError::TerminatedRetryError(WithCauseError::new(msg))
    }

    /// Creates a terminated-retry failure with a cause.
    pub fn terminated_with_cause(msg: impl Into<String>, cause: Box<dyn AnyError>) -> Self {
        RetryError::TerminatedRetryError(WithCauseError::with_cause(msg, cause))
    }

    /// Creates a back-off-interrupted failure.
    pub fn back_off_interrupted(msg: impl Into<String>) -> Self {
        RetryError::BackOffInterruptedError(WithCauseError::new(msg))
    }

    /// Creates a back-off-interrupted failure with a cause.
    pub fn back_off_interrupted_with_cause(
        msg: impl Into<String>,
        cause: Box<dyn AnyError>,
    ) -> Self {
        RetryError::BackOffInterruptedError(WithCauseError::with_cause(msg, cause))
    }

    /// Creates a cache-capacity-exceeded failure.
    pub fn cache_capacity_exceeded(msg: impl Into<String>) -> Self {
        RetryError::RetryCacheCapacityExceededError(WithCauseError::new(msg))
    }

    /// Wraps an arbitrary error.
    pub fn from_any_error(error: Box<dyn AnyError>) -> Self {
        RetryError::Any(error)
    }

    /// Returns the concrete failure kind.
    #[must_use]
    pub fn kind(&self) -> RetryErrorKind {
        match self {
            RetryError::Custom(_) => RetryErrorKind::Custom,
            RetryError::Any(_) => RetryErrorKind::Retry,
            RetryError::ExhaustedRetryError(_) => RetryErrorKind::ExhaustedRetry,
            RetryError::TerminatedRetryError(_) => RetryErrorKind::TerminatedRetry,
            RetryError::BackOffInterruptedError(_) => RetryErrorKind::BackOffInterrupted,
            RetryError::RetryCacheCapacityExceededError(_) => {
                RetryErrorKind::RetryCacheCapacityExceeded
            }
            RetryError::Default(_) => RetryErrorKind::Default,
        }
    }

    /// Returns the rendered message of this failure, without its cause.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            RetryError::Custom(msg) => msg.clone(),
            RetryError::Any(error) => error.to_string(),
            RetryError::ExhaustedRetryError(error)
            | RetryError::TerminatedRetryError(error)
            | RetryError::BackOffInterruptedError(error)
            | RetryError::RetryCacheCapacityExceededError(error)
            | RetryError::Default(error) => error.msg.clone(),
        }
    }

    /// Returns the cause of this failure, if it has one.
    #[must_use]
    pub fn cause(&self) -> Option<&dyn AnyError> {
        match self {
            RetryError::ExhaustedRetryError(error)
            | RetryError::TerminatedRetryError(error)
            | RetryError::BackOffInterruptedError(error)
            | RetryError::RetryCacheCapacityExceededError(error)
            | RetryError::Default(error) => error.cause.as_deref(),
            RetryError::Custom(_) | RetryError::Any(_) => None,
        }
    }

    /// Returns `true` when all attempts were used up.
    #[must_use]
    pub fn is_exhausted(&self) -> bool {
        matches!(self, RetryError::ExhaustedRetryError(_))
    }

    /// Returns `true` when a listener vetoed the retry.
    #[must_use]
    pub fn is_terminated(&self) -> bool {
        matches!(self, RetryError::TerminatedRetryError(_))
    }

    /// Returns `true` when waiting between attempts was interrupted.
    #[must_use]
    pub fn is_back_off_interrupted(&self) -> bool {
        matches!(self, RetryError::BackOffInterruptedError(_))
    }

    /// Returns this failure as a type-erased error.
    ///
    /// The conversion keeps the full error, so a caller that stores the result can
    /// downcast it back to [`RetryError`] later.
    #[must_use]
    pub fn as_any_error(&self) -> Option<Box<dyn AnyError>> {
        Some(Box::new(self.clone()))
    }

    /// Consumes this failure and returns it as a type-erased error.
    #[must_use]
    pub fn into_any_error(self) -> Box<dyn AnyError> {
        Box::new(self)
    }
}

impl PartialEq for RetryError {
    fn eq(&self, other: &Self) -> bool {
        self.kind() == other.kind() && self.message() == other.message()
    }
}

impl Eq for RetryError {}

impl Hash for RetryError {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind().hash(state);
        self.message().hash(state);
    }
}

impl fmt::Display for RetryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RetryError::Custom(msg) => f.write_str(msg),
            RetryError::Any(error) => write!(f, "{error}"),
            RetryError::ExhaustedRetryError(error)
            | RetryError::TerminatedRetryError(error)
            | RetryError::BackOffInterruptedError(error)
            | RetryError::RetryCacheCapacityExceededError(error)
            | RetryError::Default(error) => write!(f, "{error}"),
        }
    }
}

impl Error for RetryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            RetryError::Custom(_) => None,
            RetryError::Any(error) => error.source(),
            RetryError::ExhaustedRetryError(error)
            | RetryError::TerminatedRetryError(error)
            | RetryError::BackOffInterruptedError(error)
            | RetryError::RetryCacheCapacityExceededError(error)
            | RetryError::Default(error) => error.source(),
        }
    }
}

impl From<BoxError> for RetryError {
    fn from(error: BoxError) -> Self {
        RetryError::Custom(error.to_string())
    }
}

impl From<Box<dyn AnyError>> for RetryError {
    fn from(error: Box<dyn AnyError>) -> Self {
        // Unwrap a stored retry error so a value that was erased and recovered keeps its kind
        // and message, which is what error classification relies on.
        match error.as_any().downcast_ref::<RetryError>() {
            Some(retry_error) => retry_error.clone(),
            None => RetryError::Any(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::{RetryError, RetryErrorKind, WithCauseError};

    #[test]
    fn kind_reports_the_concrete_failure() {
        assert_eq!(RetryError::custom("x").kind(), RetryErrorKind::Custom);
        assert_eq!(
            RetryError::exhausted("x").kind(),
            RetryErrorKind::ExhaustedRetry
        );
        assert_eq!(
            RetryError::terminated("x").kind(),
            RetryErrorKind::TerminatedRetry
        );
        assert_eq!(
            RetryError::back_off_interrupted("x").kind(),
            RetryErrorKind::BackOffInterrupted
        );
        assert_eq!(
            RetryError::cache_capacity_exceeded("x").kind(),
            RetryErrorKind::RetryCacheCapacityExceeded
        );
        assert_eq!(RetryError::retry("x").kind(), RetryErrorKind::Default);
    }

    #[test]
    fn equality_and_hashing_ignore_the_cause() {
        let without_cause = RetryError::exhausted("gave up");
        let with_cause =
            RetryError::exhausted_with_cause("gave up", Box::new(RetryError::custom("inner")));

        assert_eq!(without_cause, with_cause);
        assert_ne!(RetryError::custom("a"), RetryError::custom("b"));
    }

    #[test]
    fn display_and_source_expose_the_cause_chain() {
        let error = RetryError::exhausted_with_cause(
            "gave up",
            Box::new(RetryError::custom("connection reset")),
        );

        assert_eq!(error.to_string(), "gave up: connection reset");
        let source = error.source().expect("cause should be exposed");
        assert_eq!(source.to_string(), "connection reset");
    }

    #[test]
    fn with_cause_error_reports_its_message_without_a_cause() {
        let error = WithCauseError::new("plain");

        assert_eq!(error.to_string(), "plain");
        assert!(error.source().is_none());
    }

    #[test]
    fn every_error_converts_into_the_root_error() {
        let boxed = RetryError::custom("inner").into_any_error();

        assert_eq!(RetryError::from(boxed), RetryError::custom("inner"));
    }
}
