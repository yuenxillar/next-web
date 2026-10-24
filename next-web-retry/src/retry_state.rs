//! Retry state identifies the item a stateful retry is tracking.

use crate::error::retry_error::RetryError;

/// Describes the item a stateful retry is tracking.
///
/// A stateful retry does not loop inside one call: every call fails and rethrows, and the
/// next call for the same item resumes from the stored context. The state supplies the cache
/// key that identifies the item, whether a cache lookup can be skipped, and whether a given
/// failure must roll the item back.
pub trait RetryState<K = String>
where
    Self: Send + Sync,
{
    /// Returns the cache key identifying the item, or `None` to disable caching.
    fn get_key(&self) -> Option<&K>;

    /// Returns whether the key is known to be fresh, which skips the cache lookup.
    fn is_force_refresh(&self) -> bool;

    /// Returns whether the failure must be rethrown instead of retried in place.
    ///
    /// The template calls it after a failed attempt; `true` rethrows immediately, which is
    /// what makes a stateful retry able to leave a transaction boundary.
    fn rollback_for(&self, error: &RetryError) -> bool;
}

pub use crate::support::default_retry_state::DefaultRetryState;

#[cfg(test)]
mod tests {
    use super::{DefaultRetryState, RetryState};
    use crate::error::retry_error::RetryError;

    #[test]
    fn the_default_state_is_available_through_the_trait_module() {
        let state = DefaultRetryState::new("item".to_owned());

        assert!(state.rollback_for(&RetryError::custom("any")));
        assert_eq!(state.get_key().map(String::as_str), Some("item"));
    }
}
