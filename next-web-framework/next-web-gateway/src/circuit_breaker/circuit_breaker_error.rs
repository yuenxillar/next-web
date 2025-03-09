use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum CircuitBreakerError {
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
