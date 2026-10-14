//! The states a circuit breaker moves through.

use std::fmt;

/// The state of a circuit breaker.
///
/// A breaker normally alternates between [`Self::Closed`], [`Self::Open`] and
/// [`Self::HalfOpen`]. [`Self::ForcedOpen`] and [`Self::Disabled`] are the two
/// manual overrides an operator can apply to take a guarded resource out of
/// service, or to stop the breaker from interfering with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CircuitBreakerState {
    /// Calls flow through and their outcome is recorded.
    #[default]
    Closed,
    /// Calls are rejected until the wait duration in the open state elapses.
    Open,
    /// A limited number of calls probe whether the resource has recovered.
    HalfOpen,
    /// Calls are rejected regardless of the recorded outcome.
    ForcedOpen,
    /// Calls flow through and no outcome is recorded.
    Disabled,
}

impl CircuitBreakerState {
    /// Returns whether this state lets a call through.
    pub fn is_call_permitted(self) -> bool {
        matches!(self, Self::Closed | Self::HalfOpen | Self::Disabled)
    }

    /// Returns whether this state records the outcome of the calls it lets
    /// through.
    ///
    /// A disabled breaker is transparent: it permits calls without letting
    /// their outcome influence any future transition.
    pub fn records_metrics(self) -> bool {
        matches!(self, Self::Closed | Self::HalfOpen)
    }
}

impl fmt::Display for CircuitBreakerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Closed => "CLOSED",
            Self::Open => "OPEN",
            Self::HalfOpen => "HALF_OPEN",
            Self::ForcedOpen => "FORCED_OPEN",
            Self::Disabled => "DISABLED",
        };
        f.write_str(name)
    }
}

#[cfg(test)]
mod tests {
    use super::CircuitBreakerState;

    #[test]
    fn default_state_is_closed() {
        assert_eq!(CircuitBreakerState::default(), CircuitBreakerState::Closed);
    }

    #[test]
    fn only_closed_half_open_and_disabled_permit_calls() {
        assert!(CircuitBreakerState::Closed.is_call_permitted());
        assert!(CircuitBreakerState::HalfOpen.is_call_permitted());
        assert!(CircuitBreakerState::Disabled.is_call_permitted());
        assert!(!CircuitBreakerState::Open.is_call_permitted());
        assert!(!CircuitBreakerState::ForcedOpen.is_call_permitted());
    }

    #[test]
    fn disabled_permits_calls_without_recording_them() {
        assert!(CircuitBreakerState::Disabled.is_call_permitted());
        assert!(!CircuitBreakerState::Disabled.records_metrics());
    }

    #[test]
    fn states_render_their_resilience4j_names() {
        assert_eq!(CircuitBreakerState::HalfOpen.to_string(), "HALF_OPEN");
        assert_eq!(CircuitBreakerState::ForcedOpen.to_string(), "FORCED_OPEN");
    }
}
