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

impl fmt::Display for CircuitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "Closed"),
            CircuitState::Open => write!(f, "Open"),
            CircuitState::HalfOpen => write!(f, "HalfOpen"),
        }
    }
}

impl Into<CircuitState> for u8 {
    fn into(self) -> CircuitState {
        match self {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => CircuitState::Closed,
        }
    }
}
