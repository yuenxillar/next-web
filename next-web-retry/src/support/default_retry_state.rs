//! The default retry state.

use std::{fmt, sync::Arc};

use crate::{Predicate, error::retry_error::RetryError, retry_state::RetryState};

/// The default [`RetryState`].
///
/// It stores a key, the force-refresh flag and an optional rollback classifier. Without a
/// classifier every failure is considered rollback-worthy, which is the conservative choice:
/// a stateful retry rethrows and lets the caller decide how to present the item again.
pub struct DefaultRetryState<K = String> {
    key: Option<K>,
    force_refresh: bool,
    rollback_classifier: Option<Arc<dyn Predicate<RetryError>>>,
}

impl<K> DefaultRetryState<K> {
    /// Creates a state that rolls back on every failure and checks the cache.
    pub fn new(key: K) -> Self {
        Self {
            key: Some(key),
            force_refresh: false,
            rollback_classifier: None,
        }
    }

    /// Creates a state without a key.
    ///
    /// A keyless state disables caching, so the retry can only make the attempt it is given
    /// and then rethrows.
    pub fn without_key() -> Self {
        Self {
            key: None,
            force_refresh: false,
            rollback_classifier: None,
        }
    }

    /// Creates a state from an optional key.
    pub fn with_optional_key(key: Option<K>) -> Self {
        Self {
            key,
            force_refresh: false,
            rollback_classifier: None,
        }
    }

    /// Creates a state with an explicit force-refresh flag.
    pub fn with_force_refresh(key: K, force_refresh: bool) -> Self {
        Self {
            key: Some(key),
            force_refresh,
            rollback_classifier: None,
        }
    }

    /// Creates a state from an optional key and an explicit force-refresh flag.
    pub fn with_optional_key_and_refresh(key: Option<K>, force_refresh: bool) -> Self {
        Self {
            key,
            force_refresh,
            rollback_classifier: None,
        }
    }

    /// Creates a state with a rollback classifier.
    ///
    /// The classifier answers `true` for errors that must be rethrown, which triggers a
    /// rollback of any surrounding transaction.
    pub fn with_rollback_classifier(
        key: K,
        force_refresh: bool,
        rollback_classifier: Arc<dyn Predicate<RetryError>>,
    ) -> Self {
        Self {
            key: Some(key),
            force_refresh,
            rollback_classifier: Some(rollback_classifier),
        }
    }

    /// Creates a state from an optional key and a rollback classifier.
    pub fn with_optional_key_and_rollback_classifier(
        key: Option<K>,
        force_refresh: bool,
        rollback_classifier: Option<Arc<dyn Predicate<RetryError>>>,
    ) -> Self {
        Self {
            key,
            force_refresh,
            rollback_classifier,
        }
    }

    /// Returns the key of this state, if it has one.
    pub fn key(&self) -> Option<&K> {
        self.key.as_ref()
    }

    /// Returns whether a cache lookup can be skipped.
    pub fn is_force_refresh(&self) -> bool {
        self.force_refresh
    }
}

impl<K> RetryState<K> for DefaultRetryState<K>
where
    K: Send + Sync,
{
    fn get_key(&self) -> Option<&K> {
        self.key.as_ref()
    }

    fn is_force_refresh(&self) -> bool {
        self.force_refresh
    }

    fn rollback_for(&self, error: &RetryError) -> bool {
        match &self.rollback_classifier {
            Some(classifier) => classifier.test(error),
            None => true,
        }
    }
}

impl<K: fmt::Debug> fmt::Debug for DefaultRetryState<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefaultRetryState")
            .field("key", &self.key)
            .field("force_refresh", &self.force_refresh)
            .field(
                "has_rollback_classifier",
                &self.rollback_classifier.is_some(),
            )
            .finish()
    }
}

impl<K: fmt::Display> fmt::Display for DefaultRetryState<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[DefaultRetryState: key={}, forceRefresh={}]",
            self.key
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "none".to_owned()),
            self.force_refresh
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::DefaultRetryState;
    use crate::{error::retry_error::RetryError, retry_state::RetryState};

    #[test]
    fn a_state_without_a_classifier_rolls_back_every_failure() {
        let state = DefaultRetryState::new("item-1".to_owned());

        assert!(state.rollback_for(&RetryError::custom("any")));
        assert_eq!(state.get_key().map(String::as_str), Some("item-1"));
        assert!(!state.is_force_refresh());
    }

    #[test]
    fn a_classifier_decides_which_failures_roll_back() {
        let state = DefaultRetryState::with_rollback_classifier(
            "item-1".to_owned(),
            true,
            Arc::new(|error: &RetryError| error.message() == "fatal"),
        );

        assert!(state.rollback_for(&RetryError::custom("fatal")));
        assert!(!state.rollback_for(&RetryError::custom("transient")));
        assert!(state.is_force_refresh());
    }

    #[test]
    fn display_reports_the_key_and_the_refresh_flag() {
        let state = DefaultRetryState::with_force_refresh("key".to_owned(), true);

        assert_eq!(
            state.to_string(),
            "[DefaultRetryState: key=key, forceRefresh=true]"
        );
    }
}
