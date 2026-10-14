//! The error type of a rate limiter guarded call.

use std::{error::Error, fmt};

use crate::error::RateLimitExceededError;

/// The failure of a call guarded by a [`RateLimiter`].
///
/// [`RateLimiter`]: crate::ratelimiter::RateLimiter
#[derive(Debug)]
pub enum RateLimiterError<E> {
    /// The limiter had no permission left for this call, and waiting for the
    /// next period did not fit in the configured timeout.
    ///
    /// The guarded call was never executed.
    RateLimitExceeded(RateLimitExceededError),
    /// The guarded call was executed and failed.
    Inner(E),
}

impl<E> RateLimiterError<E> {
    /// Wraps the failure of the guarded call.
    pub fn inner(value: E) -> Self {
        Self::Inner(value)
    }

    /// Returns whether the call was rejected instead of executed.
    pub fn is_rate_limit_exceeded(&self) -> bool {
        matches!(self, Self::RateLimitExceeded(_))
    }

    /// Returns the failure of the guarded call, if it was executed.
    pub fn inner_error(&self) -> Option<&E> {
        match self {
            Self::Inner(error) => Some(error),
            Self::RateLimitExceeded(_) => None,
        }
    }
}

impl<E: fmt::Display> fmt::Display for RateLimiterError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RateLimitExceeded(error) => error.fmt(f),
            Self::Inner(error) => error.fmt(f),
        }
    }
}

impl<E: Error + 'static> Error for RateLimiterError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::RateLimitExceeded(error) => Some(error),
            Self::Inner(error) => Some(error),
        }
    }
}

impl<E> From<RateLimitExceededError> for RateLimiterError<E> {
    fn from(error: RateLimitExceededError) -> Self {
        Self::RateLimitExceeded(error)
    }
}

impl<E> From<RateLimiterError<E>> for crate::error::ResilienceError<E> {
    fn from(error: RateLimiterError<E>) -> Self {
        match error {
            RateLimiterError::RateLimitExceeded(inner) => Self::RateLimitExceeded(inner),
            RateLimiterError::Inner(inner) => Self::Inner(inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::RateLimiterError;
    use crate::error::{RateLimitExceededError, ResilienceError};

    #[test]
    fn a_rejected_call_carries_no_inner_error() {
        let error: RateLimiterError<&str> = RateLimitExceededError.into();

        assert!(error.is_rate_limit_exceeded());
        assert!(error.inner_error().is_none());
    }

    #[test]
    fn an_executed_call_exposes_its_failure() {
        let error = RateLimiterError::inner("boom");

        assert!(!error.is_rate_limit_exceeded());
        assert_eq!(error.inner_error(), Some(&"boom"));
    }

    #[test]
    fn a_rejection_renders_as_the_shared_rate_limit_error() {
        let error: RateLimiterError<&str> = RateLimitExceededError.into();

        assert_eq!(error.to_string(), RateLimitExceededError.to_string());
    }

    #[test]
    fn a_rejection_converts_into_the_composed_error() {
        let error: RateLimiterError<&str> = RateLimitExceededError.into();
        let composed: ResilienceError<&str> = error.into();

        assert!(composed.is_rejection());
        assert!(matches!(composed, ResilienceError::RateLimitExceeded(_)));
    }

    #[test]
    fn a_guard_failure_converts_into_the_composed_error() {
        let error = RateLimiterError::inner("boom");
        let composed: ResilienceError<&str> = error.into();

        assert!(!composed.is_rejection());
        assert_eq!(composed.inner_error(), Some(&"boom"));
    }

    #[test]
    fn a_rejection_reports_the_shared_error_as_its_source() {
        // `Error` is only implemented for those `E` that are errors themselves,
        // so the guarded failure is a real error type here.
        let error: RateLimiterError<std::io::Error> = RateLimitExceededError.into();

        let source = error.source().expect("a source error is set");
        assert_eq!(source.to_string(), RateLimitExceededError.to_string());
    }

    #[test]
    fn a_guard_failure_reports_itself_as_the_source() {
        let error: RateLimiterError<std::io::Error> =
            RateLimiterError::inner(std::io::Error::other("boom"));

        let source = error.source().expect("a source error is set");
        assert_eq!(source.to_string(), "boom");
    }
}
