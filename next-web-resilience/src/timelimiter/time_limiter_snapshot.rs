//! A point in time view of a time limiter's counters.

/// The counters a time limiter keeps, as of one instant.
///
/// Every counter accumulates since the limiter was created and is monotonic:
/// nothing ever resets them. A call is counted exactly once, when its outcome
/// becomes known to the caller — a call abandoned by
/// [`TimeLimiter::call_non_cancelling`](crate::timelimiter::TimeLimiter::call_non_cancelling)
/// counts as a timeout at the deadline, and whatever its detached task produces
/// later is never counted again.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeLimiterSnapshot {
    /// Total number of calls that returned a value.
    pub number_of_successful_calls: u64,
    /// Total number of calls that returned an error of the guarded operation,
    /// plus the calls that ended without a result.
    pub number_of_failed_calls: u64,
    /// Total number of calls that outlived the time limit.
    pub number_of_timeouts: u64,
}

#[cfg(test)]
mod tests {
    use super::TimeLimiterSnapshot;

    #[test]
    fn counters_can_be_read_and_copied() {
        let snapshot = TimeLimiterSnapshot {
            number_of_successful_calls: 1,
            number_of_failed_calls: 2,
            number_of_timeouts: 3,
        };

        let copy = snapshot;
        assert_eq!(copy, snapshot);
        assert_eq!(copy.number_of_successful_calls, 1);
        assert_eq!(copy.number_of_failed_calls, 2);
        assert_eq!(copy.number_of_timeouts, 3);
    }
}
