//! How long a contender waits before its next acquisition attempt.

use std::time::Duration;

/// The delay before the next attempt, or `None` when the caller should stop.
///
/// Three bounds apply and the smallest wins: the time the caller has left, the
/// configured retry interval, and the TTL Redis reported for the lock that is in
/// the way. Waiting exactly for the TTL would race the release, so the retry
/// interval usually wins and the wait is woken early by Pub/Sub anyway. The
/// one-millisecond floor keeps a zero-length wait from spinning.
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
}
