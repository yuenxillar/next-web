//! Events published by a circuit breaker.

use std::time::Duration;

use super::circuit_breaker_state::CircuitBreakerState;

/// Something a circuit breaker did.
///
/// Events are delivered to every subscriber of
/// [`CircuitBreaker::event_stream`](crate::circuitbreaker::CircuitBreaker::event_stream)
/// and are meant for observability: metrics exporters, dashboards and logs. The
/// guarded call never waits for a subscriber.
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerEvent {
    /// The breaker moved from one state to another.
    StateTransition {
        /// State the breaker left.
        from: CircuitBreakerState,
        /// State the breaker entered.
        to: CircuitBreakerState,
    },
    /// A guarded call succeeded.
    Success {
        /// How long the call took.
        duration: Duration,
    },
    /// A guarded call failed with an error that counts against the failure
    /// rate.
    Error {
        /// How long the call took before failing.
        duration: Duration,
        /// Rendered failure of the call.
        message: String,
    },
    /// A guarded call failed with an error the failure predicate excludes.
    ///
    /// The call is recorded as a non-failure, so it cannot open the breaker.
    IgnoredError {
        /// How long the call took before failing.
        duration: Duration,
        /// Rendered failure of the call.
        message: String,
    },
    /// A guarded call took longer than the slow call threshold.
    ///
    /// A slow call that also failed produces both a [`Self::SlowCall`] and an
    /// [`Self::Error`] event.
    SlowCall {
        /// How long the call took.
        duration: Duration,
    },
    /// A call was rejected because the breaker does not permit calls.
    CallNotPermitted {
        /// State that rejected the call.
        state: CircuitBreakerState,
    },
    /// The failure rate of the sliding window crossed its threshold.
    FailureRateExceeded {
        /// Failure rate that triggered the transition.
        failure_rate: f32,
    },
    /// The slow call rate of the sliding window crossed its threshold.
    SlowCallRateExceeded {
        /// Slow call rate that triggered the transition.
        slow_call_rate: f32,
    },
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::CircuitBreakerEvent;
    use crate::circuitbreaker::circuit_breaker_state::CircuitBreakerState;

    #[test]
    fn events_can_be_cloned_and_compared() {
        let event = CircuitBreakerEvent::StateTransition {
            from: CircuitBreakerState::Closed,
            to: CircuitBreakerState::Open,
        };

        assert_eq!(event.clone(), event);
    }

    #[test]
    fn slow_call_events_keep_their_duration() {
        let event = CircuitBreakerEvent::SlowCall {
            duration: Duration::from_millis(250),
        };

        match event {
            CircuitBreakerEvent::SlowCall { duration } => {
                assert_eq!(duration, Duration::from_millis(250));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
