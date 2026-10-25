//! The retry policy both lock flavours share: how long to wait, and when to stop.

use std::time::{Duration, Instant};

/// How long a caller is willing to keep retrying.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Deadline {
    /// Attempt once and give up.
    Once,
    /// Retry for as long as it takes.
    Forever,
    /// Retry until this instant.
    Until(Instant),
}

impl Deadline {
    /// A deadline `wait` from now, or [`Deadline::Forever`] when the instant is
    /// not representable (a wait that long is "forever" in practice).
    pub(crate) fn after(wait: Duration) -> Self {
        match Instant::now().checked_add(wait) {
            Some(until) => Self::Until(until),
            None => Self::Forever,
        }
    }

    /// The time left, or `None` when the caller should stop.
    ///
    /// `Some(None)` means there is no bound at all.
    pub(crate) fn remaining(&self) -> Option<Option<Duration>> {
        match self {
            Self::Once => None,
            Self::Forever => Some(None),
            Self::Until(until) => {
                let now = Instant::now();
                if now >= *until {
                    None
                } else {
                    Some(Some(until.saturating_duration_since(now)))
                }
            }
        }
    }

    /// Whether a bounded deadline has already passed.
    pub(crate) fn expired(&self) -> bool {
        match self {
            Self::Until(until) => Instant::now() >= *until,
            Self::Once | Self::Forever => false,
        }
    }
}

/// The delay before the next attempt, or `None` when the caller should stop.
///
/// Three bounds apply and the smallest wins: the time the caller has left, the
/// configured retry interval, and the TTL Redis reported for the lock that is in
/// the way. The retry interval usually wins because the wait is woken early by
/// Pub/Sub anyway. The one-millisecond floor keeps a zero-length wait from
/// spinning.
pub(crate) fn next_delay(
    remaining: Option<Duration>,
    retry_interval: Duration,
    observed_ttl: Option<Duration>,
) -> Option<Duration> {
    let mut delay = retry_interval;

    if let Some(remaining) = remaining {
        if remaining.is_zero() {
            return None;
        }
        delay = delay.min(remaining);
    }

    if let Some(observed_ttl) = observed_ttl.filter(|ttl| !ttl.is_zero()) {
        delay = delay.min(observed_ttl);
    }

    Some(delay.max(Duration::from_millis(1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_retry_interval_caps_the_delay() {
        assert_eq!(
            next_delay(None, Duration::from_millis(100), None),
            Some(Duration::from_millis(100))
        );
    }

    #[test]
    fn a_short_deadline_wins_over_the_retry_interval() {
        assert_eq!(
            next_delay(
                Some(Duration::from_millis(5)),
                Duration::from_millis(100),
                None
            ),
            Some(Duration::from_millis(5))
        );
    }

    #[test]
    fn an_observed_ttl_caps_the_delay() {
        assert_eq!(
            next_delay(
                None,
                Duration::from_millis(100),
                Some(Duration::from_millis(20))
            ),
            Some(Duration::from_millis(20))
        );
    }

    #[test]
    fn a_spent_deadline_stops_the_loop_and_zero_waits_get_a_floor() {
        assert_eq!(
            next_delay(Some(Duration::ZERO), Duration::from_millis(100), None),
            None
        );
        assert_eq!(
            next_delay(None, Duration::ZERO, None),
            Some(Duration::from_millis(1))
        );
    }

    #[test]
    fn a_deadline_reports_no_time_left_once_it_has_passed() {
        let deadline = Deadline::Until(Instant::now() - Duration::from_millis(1));
        assert!(deadline.remaining().is_none());

        let unbounded = Deadline::Forever.remaining();
        assert_eq!(unbounded, Some(None));

        assert!(Deadline::Once.remaining().is_none());
    }
}
