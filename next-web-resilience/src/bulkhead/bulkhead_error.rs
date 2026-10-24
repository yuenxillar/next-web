//! The error type of a bulkhead guarded call.

use std::{error::Error, fmt};

use crate::error::BulkheadFullError;

/// The failure of a call guarded by a [`Bulkhead`].
///
/// [`Bulkhead`]: crate::bulkhead::Bulkhead
#[derive(Debug)]
pub enum BulkheadError<E> {
    /// No slot was free within the configured wait duration, so the call was
    /// refused.
    ///
    /// The guarded call was never executed.
    BulkheadFull(BulkheadFullError),
    /// The guarded call was executed and failed.
    Inner(E),
}

impl<E> BulkheadError<E> {
    /// Wraps the failure of the guarded call.
    pub fn inner(value: E) -> Self {
        Self::Inner(value)
    }

    /// Returns whether the call was rejected instead of executed.
    pub fn is_bulkhead_full(&self) -> bool {
        matches!(self, Self::BulkheadFull(_))
    }

    /// Returns the failure of the guarded call, if it was executed.
    pub fn inner_error(&self) -> Option<&E> {
        match self {
            Self::Inner(error) => Some(error),
            Self::BulkheadFull(_) => None,
        }
    }
}

impl<E: fmt::Display> fmt::Display for BulkheadError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BulkheadFull(error) => error.fmt(f),
            Self::Inner(error) => error.fmt(f),
        }
    }
}

impl<E: Error + 'static> Error for BulkheadError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::BulkheadFull(error) => Some(error),
            Self::Inner(error) => Some(error),
        }
    }
}

impl<E> From<BulkheadFullError> for BulkheadError<E> {
    fn from(error: BulkheadFullError) -> Self {
        Self::BulkheadFull(error)
    }
}

impl<E> From<BulkheadError<E>> for crate::error::ResilienceError<E> {
    fn from(error: BulkheadError<E>) -> Self {
        match error {
            BulkheadError::BulkheadFull(inner) => Self::BulkheadFull(inner),
            BulkheadError::Inner(inner) => Self::Inner(inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{error::Error, io};

    use super::BulkheadError;
    use crate::error::{BulkheadFullError, ResilienceError};

    #[test]
    fn a_rejected_call_carries_no_inner_error() {
        let error: BulkheadError<&str> = BulkheadFullError.into();

        assert!(error.is_bulkhead_full());
        assert!(error.inner_error().is_none());
    }

    #[test]
    fn an_executed_call_exposes_its_failure() {
        let error = BulkheadError::inner("boom");

        assert!(!error.is_bulkhead_full());
        assert_eq!(error.inner_error(), Some(&"boom"));
    }

    #[test]
    fn a_rejection_is_rendered_and_exposes_its_source() {
        let error: BulkheadError<io::Error> = BulkheadFullError.into();

        assert!(error.to_string().contains("bulkhead is full"));
        let source = error.source().expect("a rejection has a source");
        assert!(source.to_string().contains("bulkhead is full"));
    }

    #[test]
    fn both_variants_convert_into_a_resilience_error() {
        let rejected: BulkheadError<&str> = BulkheadFullError.into();
        let converted: ResilienceError<&str> = rejected.into();
        assert!(matches!(converted, ResilienceError::BulkheadFull(_)));

        let failed: BulkheadError<&str> = BulkheadError::inner("boom");
        let converted: ResilienceError<&str> = failed.into();
        assert_eq!(converted.inner_error(), Some(&"boom"));
    }
}
