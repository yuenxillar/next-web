use std::{error::Error, fmt};

/// Health check error
#[derive(Debug, Clone)]
pub enum HealthError {
    /// Error message
    #[allow(dead_code)]
    E(String),
}

impl fmt::Display for HealthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                HealthError::E(s) => s,
            }
        )
    }
}

impl Error for HealthError {}
