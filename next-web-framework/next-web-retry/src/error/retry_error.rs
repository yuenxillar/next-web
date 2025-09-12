use std::error::Error;

use crate::error::AnyError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RetryError {
    Custom(String),
    Any(Box<dyn AnyError>),
    ExhaustedRetryError(WithCauseError),
    Default(WithCauseError),
    TerminatedRetryError(WithCauseError),
    BackOffInterruptedError(WithCauseError),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WithCauseError {
    pub msg: String,
    pub cause: Option<Box<dyn AnyError>>,
}

impl Error for RetryError {
    
}

impl std::fmt::Display for RetryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RetryError")
    }
}