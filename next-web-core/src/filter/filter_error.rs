use std::any::Any;
use std::error::Error;
use std::fmt;
use std::io;

use crate::error::BoxError;

/// Filter chain execution error.
///
/// This type represents all errors that can occur during filter chain processing,
/// including I/O errors, downstream handler failures, HTTP protocol errors, and
/// explicit filter rejections.
#[derive(Debug)]
pub enum FilterError {
    /// An I/O error occurred during request/response processing.
    Io(io::Error),

    /// An error propagated from a downstream filter or handler in the chain.
    Chain(Box<dyn ChainError>),

    /// A general-purpose custom error with a descriptive message.
    Custom(String),
}

impl FilterError {
    /// Creates a new `FilterError::Custom` with the given message.
    ///
    /// # Arguments
    ///
    /// * `message` - A human-readable description of the error.
    pub fn custom(message: impl Into<String>) -> Self {
        FilterError::Custom(message.into())
    }

    /// Returns `true` if this is a `FilterError::Io`.
    pub fn is_io(&self) -> bool {
        matches!(self, FilterError::Io(_))
    }

    /// Returns `true` if this is a `FilterError::Chain`.
    pub fn is_chain(&self) -> bool {
        matches!(self, FilterError::Chain(_))
    }
}

impl fmt::Display for FilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilterError::Io(e) => write!(f, "I/O error: {e}"),
            FilterError::Chain(e) => write!(f, "chain error: {e}"),
            FilterError::Custom(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for FilterError {}

// ---- From conversions ----
impl From<io::Error> for FilterError {
    fn from(e: io::Error) -> Self {
        FilterError::Io(e)
    }
}

impl From<Box<dyn ChainError>> for FilterError {
    fn from(error: Box<dyn ChainError>) -> Self {
        FilterError::Chain(error)
    }
}

pub trait ChainError: Error + Send + Sync + Any {}

impl<T: Error + Send + Sync + Any> ChainError for T {}
