use std::error::Error;



#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RetryError {
    Custom(String),
    Default
}

impl Error for RetryError {
    
}

impl std::fmt::Display for RetryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RetryError")
    }
}