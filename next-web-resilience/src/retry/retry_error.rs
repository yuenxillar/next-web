//! The error type of a retried call.

use std::{error::Error, fmt};

use crate::error::MaxRetriesExceededError;

/// The failure of a call guarded by a [`Retry`].
///
/// [`Retry`]: crate::retry::Retry
#[derive(Debug)]
pub enum RetryError<E> {
    /// Every attempt failed and the configuration asks for the exhaustion to
    /// be reported instead of the last failure.
    ///
    /// This variant only appears when the configuration was built with
    /// `fail_after_max_attempts(true)`.
    MaxRetriesExceeded(MaxRetriesExceededError<E>),
    /// The guarded call failed and the retry policy stopped.
    ///
    /// This is either the failure of the last attempt after the attempt budget
    /// was spent, or a failure the policy never retries.
    Inner(E),
}

impl<E> RetryError<E> {
    /// Wraps the failure of the guarded call.
    pub fn inner(value: E) -> Self {
        Self::Inner(value)
    }

    /// Returns whether the attempt budget was reported as spent.
    pub fn is_max_retries_exceeded(&self) -> bool {
        matches!(self, Self::MaxRetriesExceeded(_))
    }

    /// Returns the number of attempts made, when the budget was reported as
    /// spent.
    pub fn attempts(&self) -> Option<u32> {
        match self {
            Self::MaxRetriesExceeded(error) => Some(error.attempts),
            Self::Inner(_) => None,
        }
    }

    /// Returns the failure of the guarded call, if it was preserved.
    pub fn inner_error(&self) -> Option<&E> {
        match self {
            Self::MaxRetriesExceeded(error) => error.last_error.as_ref(),
            Self::Inner(error) => Some(error),
        }
    }

    /// Consumes this error and returns the failure of the guarded call.
    pub fn into_inner(self) -> Option<E> {
        match self {
            Self::MaxRetriesExceeded(error) => error.last_error,
            Self::Inner(error) => Some(error),
        }
    }
}

impl<E: fmt::Display> fmt::Display for RetryError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MaxRetriesExceeded(error) => error.fmt(f),
            Self::Inner(error) => error.fmt(f),
        }
    }
}

impl<E: Error + 'static> Error for RetryError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::MaxRetriesExceeded(error) => Some(error),
            Self::Inner(error) => Some(error),
        }
    }
}

impl<E> From<MaxRetriesExceededError<E>> for RetryError<E> {
    fn from(error: MaxRetriesExceededError<E>) -> Self {
        Self::MaxRetriesExceeded(error)
    }
}

impl<E> From<RetryError<E>> for crate::error::ResilienceError<E> {
    fn from(error: RetryError<E>) -> Self {
        match error {
            RetryError::MaxRetriesExceeded(inner) => Self::MaxRetriesExceeded(inner),
            RetryError::Inner(inner) => Self::Inner(inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RetryError;
    use crate::error::MaxRetriesExceededError;

    #[test]
    fn an_exhausted_budget_reports_its_attempts_and_last_failure() {
        let error: RetryError<&str> =
            MaxRetriesExceededError::new(3, Some("boom")).into();

        assert!(error.is_max_retries_exceeded());
        assert_eq!(error.attempts(), Some(3));
        assert_eq!(error.inner_error(), Some(&"boom"));
    }

    #[test]
    fn a_plain_failure_reports_no_attempt_count() {
        let error = RetryError::inner("boom");

        assert!(!error.is_max_retries_exceeded());
        assert_eq!(error.attempts(), None);
        assert_eq!(error.inner_error(), Some(&"boom"));
    }

    #[test]
    fn into_inner_moves_the_failure_out_of_an_exhausted_budget() {
        let error: RetryError<String> =
            MaxRetriesExceededError::new(2, Some("boom".to_owned())).into();

        assert_eq!(error.into_inner(), Some("boom".to_owned()));
    }

    #[test]
    fn an_exhausted_budget_without_a_failure_has_no_inner_error() {
        let error: RetryError<&str> = MaxRetriesExceededError::new(2, None).into();

        assert!(error.inner_error().is_none());
        assert!(error.to_string().contains('2'));
    }
}
