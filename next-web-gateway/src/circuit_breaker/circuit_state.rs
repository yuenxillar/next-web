use std::fmt;

/// Represents the state of a circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    /// The circuit is closed and allowing requests to pass through.
    Closed,
    /// The circuit is open and blocking requests.
    Open,
    /// The circuit is allowing a limited number of requests to test if the system has recovered.
    HalfOpen,
}

impl From<CircuitState> for u8 {
    fn from(state: CircuitState) -> Self {
        match state {
            CircuitState::Closed => 0,
            CircuitState::Open => 1,
            CircuitState::HalfOpen => 2,
        }
    }
}

impl fmt::Display for CircuitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "Closed"),
            CircuitState::Open => write!(f, "Open"),
            CircuitState::HalfOpen => write!(f, "HalfOpen"),
        }
    }
}

impl From<u8> for CircuitState {
    fn from(value: u8) -> Self {
        match value {
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => CircuitState::Closed,
        }
    }
}
