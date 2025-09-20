use std::error::Error;

use crate::error::AnyError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RetryError {
    Custom(String),
    Any(Box<dyn AnyError>),
    ExhaustedRetryError(WithCauseError),
    TerminatedRetryError(WithCauseError),
    BackOffInterruptedError(WithCauseError),
    Default(WithCauseError),
    Parent
}


impl RetryError {
    pub fn as_any_error(& self) -> Option<Box<dyn AnyError>> {
        match self {
            RetryError::Any(any_error) => Some(any_error.clone()),
            RetryError::Custom(msg) => Some(Box::new(DefaultAnyError(WithCauseError { msg: msg.to_string(), cause: None }))),
            RetryError::ExhaustedRetryError(error) => Some(Box::new(DefaultAnyError(error.clone()))),
            RetryError::Default(error) => Some(Box::new(DefaultAnyError(error.clone()))),
            RetryError::TerminatedRetryError(error) => Some(Box::new(DefaultAnyError(error.clone()))),
            RetryError::BackOffInterruptedError(error) => Some(Box::new(DefaultAnyError(error.clone()))),
            RetryError::Parent => Some(
                Box::new(DefaultAnyError(WithCauseError {
                    msg: "Retry Parent Error".to_string(),
                    cause: None,
                }))
            )
        }
    }
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

#[derive(Clone, Debug, Hash)]
pub struct DefaultAnyError(pub WithCauseError);

impl std::error::Error for DefaultAnyError {}

impl std::fmt::Display for DefaultAnyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DefaultAnyError")
    }
}