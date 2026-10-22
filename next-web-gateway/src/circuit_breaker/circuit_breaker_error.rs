use std::error::Error;
use std::fmt;

/// Error returned when a breaker refuses a request.
#[derive(Debug)]
pub enum CircuitBreakerError {
    /// The breaker is open and will not admit the request.
    CircuitOpen,
}

impl Error for CircuitBreakerError {}

impl fmt::Display for CircuitBreakerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CircuitBreakerError::CircuitOpen => write!(f, "Circuit is open"),
        }
    }
}
