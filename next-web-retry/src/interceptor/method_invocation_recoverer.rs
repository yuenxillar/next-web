//! Produces the return value of an intercepted call whose retry is exhausted.

use next_web_core::any::AnyValue;

use crate::error::retry_error::RetryError;

/// Produces the return value of an intercepted call whose retry was exhausted.
///
/// The recoverer receives the arguments of the failed call and the failure that exhausted the
/// retry, and returns the value the caller should see instead. It is the interceptor
/// counterpart of [`RecoveryCallback`](crate::recovery_callback::RecoveryCallback).
pub trait MethodInvocationRecoverer<T>
where
    Self: Send + Sync,
{
    /// Recovers from a failure.
    fn recover(&self, arguments: &[AnyValue], cause: Option<&RetryError>) -> Result<T, RetryError>;
}

impl<T, F> MethodInvocationRecoverer<T> for F
where
    F: Fn(&[AnyValue], Option<&RetryError>) -> Result<T, RetryError> + Send + Sync,
{
    fn recover(&self, arguments: &[AnyValue], cause: Option<&RetryError>) -> Result<T, RetryError> {
        self(arguments, cause)
    }
}

/// A recoverer that always produces the same value.
pub struct ConstantRecoverer<T> {
    value: T,
}

impl<T> ConstantRecoverer<T> {
    /// Creates a recoverer that returns `value`.
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T> MethodInvocationRecoverer<T> for ConstantRecoverer<T>
where
    T: Clone + Send + Sync,
{
    fn recover(
        &self,
        _arguments: &[AnyValue],
        _cause: Option<&RetryError>,
    ) -> Result<T, RetryError> {
        Ok(self.value.clone())
    }
}

#[cfg(test)]
mod tests {
    use next_web_core::any::AnyValue;

    use super::{ConstantRecoverer, MethodInvocationRecoverer};
    use crate::error::retry_error::RetryError;

    #[test]
    fn a_closure_recoverer_sees_the_arguments_and_the_cause() {
        let recoverer = |arguments: &[AnyValue], cause: Option<&RetryError>| {
            let argument = arguments
                .first()
                .and_then(|value| value.as_u64())
                .unwrap_or_default();
            let sum = argument + u64::from(cause.is_some());
            Ok(sum)
        };

        let recovered = recoverer
            .recover(&[AnyValue::from(41_u64)], Some(&RetryError::custom("boom")))
            .expect("recovery should succeed");

        assert_eq!(recovered, 42);
    }

    #[test]
    fn a_constant_recoverer_returns_its_value() {
        let recoverer = ConstantRecoverer::new("fallback");

        assert_eq!(
            recoverer
                .recover(&[], None)
                .expect("recovery should succeed"),
            "fallback"
        );
    }
}
