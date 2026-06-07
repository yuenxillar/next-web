use core::fmt;
use std::error::Error;

#[derive(Debug)]
pub struct AccessDeniedError(pub String);

impl fmt::Display for AccessDeniedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Access denied: {}", self.0)
    }
}

impl Error for AccessDeniedError {}
