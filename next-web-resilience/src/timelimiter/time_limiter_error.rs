//! The error type of a time limiter guarded call.

use std::{error::Error, fmt};

use crate::error::{AbortedError, TimeoutError};

/// The failure of a call guarded by a [`TimeLimiter`].
///
/// [`TimeLimiter`]: crate::timelimiter::TimeLimiter
#[derive(Debug)]
pub enum TimeLimiterError<E> {
    /// The call outlived the configured timeout.
    ///
    /// The guarded future was dropped, unless the call was made with
    /// [`call_non_cancelling`](crate::timelimiter::TimeLimiter::call_non_cancelling).
    Timeout(TimeoutError),
    /// The call ended without producing a result.
    ///
    /// This is raised by
    /// [`call_non_cancelling`](crate::timelimiter::TimeLimiter::call_non_cancelling)
    /// when the spawned task it waited for panicked, so no value and no error of
    /// the guarded operation exist; the message describes what the runtime
    /// reported. A task cancelled by the runtime ends the same way.
    Aborted(AbortedError),
    /// The guarded call was executed and failed.
    Inner(E),
}

impl<E> TimeLimiterError<E> {
    /// Wraps the failure of the guarded call.
    pub fn inner(value: E) -> Self {
        Self::Inner(value)
    }

    /// Returns whether the call outlived its time limit.
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout(_))
    }

    /// Returns the failure of the guarded call, if it produced one.
    ///
    /// Returns `None` for a timeout and for a call that ended without a result,
    /// because neither leaves a failure of the guarded operation behind.
    pub fn inner_error(&self) -> Option<&E> {
        match self {
            Self::Inner(error) => Some(error),
            Self::Timeout(_) | Self::Aborted(_) => None,
        }
    }
}

impl<E: fmt::Display> fmt::Display for TimeLimiterError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout(error) => error.fmt(f),
            Self::Aborted(error) => error.fmt(f),
            Self::Inner(error) => error.fmt(f),
        }
    }
}

impl<E: Error + 'static> Error for TimeLimiterError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Timeout(error) => Some(error),
            // The guarded operation left no failure behind, but the runtime's
            // report of what happened to the task is one.
            Self::Aborted(error) => Some(error),
            Self::Inner(error) => Some(error),
        }
    }
}

impl<E> From<TimeoutError> for TimeLimiterError<E> {
    fn from(error: TimeoutError) -> Self {
        Self::Timeout(error)
    }
}

#[cfg(test)]
mod tests {
    use std::{error::Error, time::Duration};

    use super::TimeLimiterError;
    use crate::error::{AbortedError, TimeoutError};

    #[test]
    fn a_timeout_error_converts_into_the_time_limiter_error() {
        let error: TimeLimiterError<&str> = TimeoutError::new(Duration::from_secs(2)).into();

        assert!(error.is_timeout());
        assert!(error.inner_error().is_none());
        assert_eq!(
            error.to_string(),
            TimeoutError::new(Duration::from_secs(2)).to_string()
        );
    }

    #[test]
    fn an_executed_call_exposes_its_failure_as_source() {
        let error = TimeLimiterError::Inner("boom");

        assert!(!error.is_timeout());
        assert_eq!(error.inner_error(), Some(&"boom"));
        assert_eq!(error.to_string(), "boom");
    }

    #[test]
    fn an_aborted_call_carries_the_runtime_report() {
        let error: TimeLimiterError<std::io::Error> =
            TimeLimiterError::Aborted(AbortedError::new("task 7 panicked"));

        assert!(!error.is_timeout());
        assert!(error.inner_error().is_none());
        assert!(error.to_string().contains("task 7 panicked"));
        assert_eq!(error.source().map(ToString::to_string), Some("call ended without a result: task 7 panicked".to_owned()));
    }
}
