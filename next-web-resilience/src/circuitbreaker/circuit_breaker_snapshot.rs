//! A point in time view of a circuit breaker's counters.

use super::circuit_breaker_state::CircuitBreakerState;

/// The counters a circuit breaker keeps, as of one instant.
///
/// The rates and the buffered call counts describe the sliding window, so they
/// shrink again as old calls age out. The totals — successful calls, ignored
/// errors and rejected calls — accumulate since the breaker was created and are
/// never reset by a state transition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircuitBreakerSnapshot {
    /// State the breaker is in, after any time based transition has been
    /// applied.
    pub state: CircuitBreakerState,
    /// Percentage of failed calls inside the sliding window.
    pub failure_rate: f32,
    /// Percentage of slow calls inside the sliding window.
    pub slow_call_rate: f32,
    /// Number of calls currently inside the sliding window.
    pub number_of_buffered_calls: u64,
    /// Number of those buffered calls that counted as failures.
    pub number_of_failed_calls: u64,
    /// Number of those buffered calls that counted as slow.
    pub number_of_slow_calls: u64,
    /// Total number of calls rejected because the breaker was open.
    pub number_of_not_permitted_calls: u64,
    /// Total number of successful calls.
    pub number_of_successful_calls: u64,
    /// Total number of failures excluded by the failure predicate.
    pub number_of_ignored_errors: u64,
}

impl CircuitBreakerSnapshot {
    /// Returns whether the breaker currently rejects calls.
    pub fn is_rejecting_calls(&self) -> bool {
        !self.state.is_call_permitted()
    }
}

#[cfg(test)]
mod tests {
    use super::CircuitBreakerSnapshot;
    use crate::circuitbreaker::circuit_breaker_state::CircuitBreakerState;

    fn snapshot(state: CircuitBreakerState) -> CircuitBreakerSnapshot {
        CircuitBreakerSnapshot {
            state,
            failure_rate: 0.0,
            slow_call_rate: 0.0,
            number_of_buffered_calls: 0,
            number_of_failed_calls: 0,
            number_of_slow_calls: 0,
            number_of_not_permitted_calls: 0,
            number_of_successful_calls: 0,
            number_of_ignored_errors: 0,
        }
    }

    #[test]
    fn an_open_breaker_reports_rejecting_calls() {
        assert!(snapshot(CircuitBreakerState::Open).is_rejecting_calls());
        assert!(!snapshot(CircuitBreakerState::Closed).is_rejecting_calls());
    }
}
