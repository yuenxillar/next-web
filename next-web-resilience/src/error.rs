//! Errors shared by the resilience patterns.
//!
//! Every pattern distinguishes two kinds of failure:
//!
//! * a **rejection** raised by the pattern itself — the circuit is open, the
//!   rate limiter has no permission left, the bulkhead is full, the call timed
//!   out — and
//! * the **inner** error of the guarded call.
//!
//! Each module models that with its own generic error enum built from the
//! rejection types below, and [`ResilienceError`] is the union used by the
//! composed decorator, which can fail in any of those ways.

use std::{error::Error, fmt, time::Duration};

/// Raised when a circuit breaker is open and refuses to let a call through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("circuit breaker is open and does not permit further calls")]
pub struct CallNotPermittedError;

/// Raised when a rate limiter has spent its permissions for the current period
/// and the caller asked not to wait for the next one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("rate limiter does not permit further calls in the current period")]
pub struct RateLimitExceededError;

/// Raised when a bulkhead has no permit left for another concurrent call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("bulkhead is full and does not permit further concurrent calls")]
pub struct BulkheadFullError;

/// Raised when a call outlives the time limit it was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("call exceeded the time limit of {timeout:?}")]
pub struct TimeoutError {
    /// The limit the call exceeded.
    pub timeout: Duration,
}

impl TimeoutError {
    /// Creates a timeout error for the limit that was exceeded.
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }
}

/// Raised when a guarded call ended without producing either a value or a
/// failure of the guarded operation.
///
/// This happens when the runtime reports that the task the call was running in
/// panicked or was cancelled, so there is no value and no failure to hand back.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("call ended without a result: {message}")]
pub struct AbortedError {
    /// What the runtime reported about the task that ended.
    pub message: String,
}

impl AbortedError {
    /// Creates the error from the runtime's report.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Raised when a retried call keeps failing until the attempt budget is spent.
///
/// The failure of the final attempt is preserved in [`Self::last_error`], so a
/// caller can still inspect why every attempt failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaxRetriesExceededError<E> {
    /// Number of attempts that were made.
    pub attempts: u32,
    /// Failure of the final attempt, when that attempt produced an error.
    pub last_error: Option<E>,
}

impl<E> MaxRetriesExceededError<E> {
    /// Creates the error for a spent attempt budget.
    pub fn new(attempts: u32, last_error: Option<E>) -> Self {
        Self {
            attempts,
            last_error,
        }
    }
}

impl<E> fmt::Display for MaxRetriesExceededError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "maximum number of retry attempts ({}) exceeded",
            self.attempts
        )
    }
}

impl<E: Error + 'static> Error for MaxRetriesExceededError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.last_error.as_ref().map(|error| error as &(dyn Error + 'static))
    }
}

/// The union of every failure a composed resilience decorator can report.
///
/// The variants before [`Self::Inner`] are rejections produced by a pattern;
/// [`Self::Inner`] carries the failure of the guarded call itself. Use
/// [`Self::inner_error`] to tell the two apart.
#[derive(Debug)]
pub enum ResilienceError<E> {
    /// The circuit breaker is open and blocked the call.
    CallNotPermitted(CallNotPermittedError),
    /// The rate limiter had no permission left for this call.
    RateLimitExceeded(RateLimitExceededError),
    /// The bulkhead had no permit left for this call.
    BulkheadFull(BulkheadFullError),
    /// The call exceeded the time limit enforced on it.
    Timeout(TimeoutError),
    /// The call ended without producing a result.
    ///
    /// This happens when a guarded task panicked or was cancelled, so neither a
    /// value nor a failure of the guarded operation exists.
    Aborted(AbortedError),
    /// Every retry attempt failed.
    MaxRetriesExceeded(MaxRetriesExceededError<E>),
    /// The guarded call itself failed.
    Inner(E),
}

impl<E> ResilienceError<E> {
    /// Wraps the failure of the guarded call.
    pub fn inner(value: E) -> Self {
        Self::Inner(value)
    }

    /// Returns the failure of the guarded call, if this is one.
    ///
    /// Returns `None` when the failure was a rejection raised by a pattern.
    pub fn inner_error(&self) -> Option<&E> {
        match self {
            Self::Inner(error) => Some(error),
            _ => None,
        }
    }

    /// Consumes this error and returns the failure of the guarded call.
    pub fn into_inner(self) -> Option<E> {
        match self {
            Self::Inner(error) => Some(error),
            _ => None,
        }
    }

    /// Returns `true` when a resilience pattern ended the call rather than the
    /// guarded call failing on its own.
    pub fn is_rejection(&self) -> bool {
        !matches!(self, Self::Inner(_))
    }
}

impl<E: fmt::Display> fmt::Display for ResilienceError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CallNotPermitted(error) => error.fmt(f),
            Self::RateLimitExceeded(error) => error.fmt(f),
            Self::BulkheadFull(error) => error.fmt(f),
            Self::Timeout(error) => error.fmt(f),
            Self::Aborted(error) => error.fmt(f),
            Self::MaxRetriesExceeded(error) => error.fmt(f),
            Self::Inner(error) => error.fmt(f),
        }
    }
}

impl<E: Error + 'static> Error for ResilienceError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CallNotPermitted(error) => Some(error),
            Self::RateLimitExceeded(error) => Some(error),
            Self::BulkheadFull(error) => Some(error),
            Self::Timeout(error) => Some(error),
            // A call that panicked or was cancelled leaves no failure of the
            // guarded operation to chain to, but the runtime's report is one.
            Self::Aborted(error) => Some(error),
            Self::MaxRetriesExceeded(error) => Some(error),
            Self::Inner(error) => Some(error),
        }
    }
}

impl<E> From<CallNotPermittedError> for ResilienceError<E> {
    fn from(error: CallNotPermittedError) -> Self {
        Self::CallNotPermitted(error)
    }
}

impl<E> From<RateLimitExceededError> for ResilienceError<E> {
    fn from(error: RateLimitExceededError) -> Self {
        Self::RateLimitExceeded(error)
    }
}

impl<E> From<BulkheadFullError> for ResilienceError<E> {
    fn from(error: BulkheadFullError) -> Self {
        Self::BulkheadFull(error)
    }
}

impl<E> From<TimeoutError> for ResilienceError<E> {
    fn from(error: TimeoutError) -> Self {
        Self::Timeout(error)
    }
}

impl<E> From<AbortedError> for ResilienceError<E> {
    fn from(error: AbortedError) -> Self {
        Self::Aborted(error)
    }
}

impl<E> From<MaxRetriesExceededError<E>> for ResilienceError<E> {
    fn from(error: MaxRetriesExceededError<E>) -> Self {
        Self::MaxRetriesExceeded(error)
    }
}

#[cfg(test)]
mod tests {
    use std::{error::Error, time::Duration};

    use super::{MaxRetriesExceededError, ResilienceError, TimeoutError};

    #[test]
    fn inner_error_is_available_only_for_guard_failures() {
        let failure: ResilienceError<&str> = ResilienceError::Inner("boom");

        assert_eq!(failure.inner_error(), Some(&"boom"));
        assert!(!failure.is_rejection());
    }

    #[test]
    fn rejections_report_no_inner_error() {
        let failure: ResilienceError<&str> = TimeoutError::new(Duration::from_secs(1)).into();

        assert!(failure.inner_error().is_none());
        assert!(failure.is_rejection());
        assert!(matches!(failure, ResilienceError::Timeout(_)));
    }

    #[test]
    fn exhausted_retries_expose_the_last_failure_as_source() {
        let last = TimeoutError::new(Duration::from_millis(5));
        let exhausted = MaxRetriesExceededError::new(3, Some(last));
        let failure: ResilienceError<TimeoutError> = exhausted.into();

        // The pipeline error points at the exhausted-retry error...
        let source = failure.source().expect("a source error is set");
        assert!(source.to_string().contains('3'));

        // ...which in turn points at the failure of the last attempt.
        let nested = source.source().expect("the last failure is kept");
        assert_eq!(nested.to_string(), last.to_string());
    }

    #[test]
    fn into_inner_moves_the_guard_failure_out() {
        let failure: ResilienceError<String> = ResilienceError::Inner("boom".to_owned());

        assert_eq!(failure.into_inner(), Some("boom".to_owned()));
    }
}
