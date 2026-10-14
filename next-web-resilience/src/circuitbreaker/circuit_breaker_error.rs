//! The error type of a circuit breaker guarded call.

use std::{error::Error, fmt};

use crate::error::CallNotPermittedError;

/// The failure of a call guarded by a [`CircuitBreaker`].
///
/// [`CircuitBreaker`]: crate::circuitbreaker::CircuitBreaker
#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    /// The breaker is open, or was forced open, and refused the call.
    ///
    /// The guarded call was never executed.
    CallNotPermitted(CallNotPermittedError),
    /// The guarded call was executed and failed.
    Inner(E),
}

impl<E> CircuitBreakerError<E> {
    /// Wraps the failure of the guarded call.
    pub fn inner(value: E) -> Self {
        Self::Inner(value)
    }

    /// Returns whether the call was blocked instead of executed.
    pub fn is_call_not_permitted(&self) -> bool {
        matches!(self, Self::CallNotPermitted(_))
    }

    /// Returns the failure of the guarded call, if it was executed.
    pub fn inner_error(&self) -> Option<&E> {
        match self {
            Self::Inner(error) => Some(error),
            Self::CallNotPermitted(_) => None,
        }
    }
}

impl<E: fmt::Display> fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CallNotPermitted(error) => error.fmt(f),
            Self::Inner(error) => error.fmt(f),
        }
    }
}

impl<E: Error + 'static> Error for CircuitBreakerError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CallNotPermitted(error) => Some(error),
            Self::Inner(error) => Some(error),
        }
    }
}

impl<E> From<CallNotPermittedError> for CircuitBreakerError<E> {
    fn from(error: CallNotPermittedError) -> Self {
        Self::CallNotPermitted(error)
    }
}

impl<E> From<CircuitBreakerError<E>> for crate::error::ResilienceError<E> {
    fn from(error: CircuitBreakerError<E>) -> Self {
        match error {
            CircuitBreakerError::CallNotPermitted(inner) => Self::CallNotPermitted(inner),
            CircuitBreakerError::Inner(inner) => Self::Inner(inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CircuitBreakerError;
    use crate::error::CallNotPermittedError;

    #[test]
    fn a_blocked_call_carries_no_inner_error() {
        let error: CircuitBreakerError<&str> = CallNotPermittedError.into();

        assert!(error.is_call_not_permitted());
        assert!(error.inner_error().is_none());
    }

    #[test]
    fn an_executed_call_exposes_its_failure() {
        let error = CircuitBreakerError::inner("boom");

        assert!(!error.is_call_not_permitted());
        assert_eq!(error.inner_error(), Some(&"boom"));
    }
}
