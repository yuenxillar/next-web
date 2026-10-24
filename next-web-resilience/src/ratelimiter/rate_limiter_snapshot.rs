//! A point in time view of a rate limiter's counters.

use std::time::Duration;

/// The counters a rate limiter keeps, as of one instant.
///
/// The acquired and rejected totals describe the limiter instance they came
/// from. The available permissions describe its store, and are only known when
/// that store is local — see [`Self::available_permissions`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimiterSnapshot {
    /// Permissions still available in the current period, when the store can
    /// tell without a round trip.
    ///
    /// A store local to this process answers exactly, and the value already
    /// accounts for the passage of time: once the current period is over, the
    /// whole limit is reported as available again. A store in another process
    /// reports `None`, because answering would cost a network round trip and the
    /// method that produces this snapshot does not await.
    pub available_permissions: Option<u32>,
    /// Number of permissions granted at the start of every period.
    pub limit_for_period: u32,
    /// Length of one refresh period.
    pub limit_refresh_period: Duration,
    /// Total number of permissions this instance was granted.
    pub number_of_acquired_permissions: u64,
    /// Total number of permissions this instance was refused.
    pub number_of_rejected_permissions: u64,
}

impl RateLimiterSnapshot {
    /// Returns whether the store would grant a permission right now, when it can
    /// tell.
    ///
    /// `None` means the store cannot answer locally. That is deliberately not
    /// the same as `Some(false)`: not knowing must never be read as "nothing
    /// left".
    pub fn is_permission_available(&self) -> Option<bool> {
        self.available_permissions.map(|available| available > 0)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::RateLimiterSnapshot;

    fn snapshot(available_permissions: Option<u32>) -> RateLimiterSnapshot {
        RateLimiterSnapshot {
            available_permissions,
            limit_for_period: 10,
            limit_refresh_period: Duration::from_millis(500),
            number_of_acquired_permissions: 0,
            number_of_rejected_permissions: 0,
        }
    }

    #[test]
    fn a_permission_is_available_while_the_budget_lasts() {
        assert_eq!(snapshot(Some(1)).is_permission_available(), Some(true));
    }

    #[test]
    fn a_spent_budget_offers_no_permission() {
        assert_eq!(snapshot(Some(0)).is_permission_available(), Some(false));
    }

    #[test]
    fn a_store_that_cannot_tell_reports_unknown_rather_than_none_left() {
        assert_eq!(snapshot(None).is_permission_available(), None);
    }

    #[test]
    fn snapshots_can_be_copied_and_compared() {
        let original = snapshot(Some(3));
        let copy = original;
        let same = snapshot(Some(3));

        assert_eq!(original, same);
        assert_eq!(copy.available_permissions, Some(3));
        assert_eq!(original.limit_for_period, 10);
    }
}
