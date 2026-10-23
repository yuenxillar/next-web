//! Retry templates, policies and back-off strategies.
//!
//! The crate is a Rust port of the retry semantics of the original library: an operation is
//! executed through a [`support::retry_template::RetryTemplate`], which asks a
//! [`retry_policy::RetryPolicy`] whether another attempt is allowed and a
//! [`backoff::back_off_policy::BackOffPolicy`] how long to wait before it.
//!
//! ```no_run
//! use next_web_retry::{
//!     error::retry_error::RetryError,
//!     retry_callback::with_fn,
//!     retry_operations::RetryOperations,
//!     support::retry_template::RetryTemplate,
//! };
//!
//! # async fn run() -> Result<(), RetryError> {
//! let template = RetryTemplate::builder()
//!     .max_attempts(3)
//!     .fixed_backoff(100)
//!     .build();
//!
//! let value = template
//!     .execute(with_fn(|_context| {
//!         Box::pin(async { Ok::<_, RetryError>(42) })
//!     }))
//!     .await?;
//!
//! assert_eq!(value, 42);
//! # Ok(())
//! # }
//! ```

pub mod backoff;
pub mod classifier;
pub mod context;
pub mod error;
pub mod interceptor;
pub mod listener;
pub mod policy;
pub mod recovery_callback;
pub mod retry_callback;
pub mod retry_context;
pub mod retry_listener;
pub mod retry_operations;
pub mod retry_policy;
pub mod retry_state;
pub mod stats;
pub mod support;

pub use classifier::ErrorTypeClassifier;
pub use error::retry_error::{RetryError, RetryErrorKind};
pub use recovery_callback::RecoveryCallback;
pub use retry_callback::RetryCallback;
pub use retry_context::RetryContext;
pub use retry_listener::RetryListener;
pub use retry_operations::RetryOperations;
pub use retry_policy::RetryPolicy;
pub use retry_state::RetryState;
pub use stats::{MutableRetryStatistics, RetryStatistics};
pub use support::metrics_retry_listener::{
    MetricsRetryListener, RetryMetricsRecorder, RetryObservation,
};
pub use support::retry_metrics::{RetryMetrics, RetryMetricsSnapshot, RetryOutcome};
pub use support::retry_template::RetryTemplate;

/// A synchronous predicate over a value.
///
/// Retry configuration filters are expressed as predicates because they decide, without doing
/// any work, whether a value belongs to a set. A closure with the right signature implements it
/// automatically.
///
/// ```
/// use std::sync::Arc;
///
/// use next_web_retry::{Predicate, error::retry_error::RetryError};
///
/// let retryable: Arc<dyn Predicate<RetryError>> = Arc::new(|error: &RetryError| {
///     error.message().contains("timeout")
/// });
///
/// assert!(retryable.test(&RetryError::custom("request timeout")));
/// ```
pub trait Predicate<T>
where
    Self: Send + Sync,
{
    /// Returns whether `value` satisfies this predicate.
    fn test(&self, value: &T) -> bool;
}

impl<T, F> Predicate<T> for F
where
    F: Fn(&T) -> bool + Send + Sync,
{
    fn test(&self, value: &T) -> bool {
        self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::Predicate;

    #[test]
    fn a_closure_implements_the_predicate() {
        let predicate = |value: &u32| *value > 3;

        assert!(predicate.test(&4));
        assert!(!predicate.test(&2));
    }
}
