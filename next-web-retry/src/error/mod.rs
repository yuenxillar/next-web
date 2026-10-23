//! Errors raised by the retry machinery.
//!
//! Every exception the Java implementation declares has a counterpart here:
//!
//! * [`retry_error::RetryError`] is the root error and the single error type the
//!   public API returns. It carries a [`retry_error::RetryErrorKind`] so a caller can
//!   branch on the concrete failure without matching every variant.
//! * [`exhausted_retry_error::ExhaustedRetryError`],
//!   [`terminated_retry_error::TerminatedRetryError`],
//!   [`back_off_interrupted_error::BackOffInterruptedError`] and
//!   [`retry_cache_capacity_exceeded_error::RetryCacheCapacityExceededError`] are the
//!   specialisations that give those failures a name and a cause chain.
//!
//! The specialisations convert into [`retry_error::RetryError`] with [`From`], so a
//! policy, sleeper or cache implementation can build whichever type is most descriptive
//! and still return the crate's single error type.

pub mod back_off_interrupted_error;
pub mod exhausted_retry_error;
pub mod retry_cache_capacity_exceeded_error;
pub mod retry_error;
pub mod terminated_retry_error;

pub use back_off_interrupted_error::BackOffInterruptedError;
pub use exhausted_retry_error::ExhaustedRetryError;
pub use retry_cache_capacity_exceeded_error::RetryCacheCapacityExceededError;
pub use retry_error::{RetryError, RetryErrorKind, WithCauseError};
pub use terminated_retry_error::TerminatedRetryError;
