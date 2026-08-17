use std::any::Any;
use std::error::Error;
use std::fmt;
use std::io;

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
    Chain(FilterChainError),

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

/// Error type for filter chain operations.
///
/// Provides two variants for flexible error handling:
/// - `Boxed`: Wraps standard error types without `Any` support
/// - `AnyError`: Wraps error types with full `Any` support for downcasting
#[derive(Debug)]
pub enum FilterChainError {
    /// Wraps a standard dynamic error type.
    ///
    /// This variant is suitable for general error propagation where downcasting
    /// to concrete types is not required. It implements `Send + Sync + 'static`
    /// for thread-safe error handling.
    ///
    /// # Examples
    /// ```
    /// use std::io;
    /// let err = FilterChainError::Boxed(Box::new(io::Error::new(
    ///     io::ErrorKind::NotFound,
    ///     "file not found"
    /// )));
    /// ```
    Boxed(Box<dyn Error + Send + Sync>),

    /// Wraps a dynamic error type with `Any` support.
    ///
    /// This variant enables runtime type introspection and downcasting to concrete
    /// error types. Useful when different error types need to be distinguished
    /// and handled differently at runtime.
    ///
    /// # Examples
    /// ```
    /// #[derive(Debug, thiserror::Error)]
    /// #[error("custom error")]
    /// struct CustomError;
    ///
    /// let err = FilterChainError::AnyError(Box::new(CustomError));
    /// if err.is::<CustomError>() {
    ///     // Handle custom error specifically
    /// }
    /// ```
    AnyError(Box<dyn ChainError>),
}

impl fmt::Display for FilterChainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilterChainError::Boxed(e) => write!(f, "{}", e),
            FilterChainError::AnyError(e) => write!(f, "{}", e),
        }
    }
}

impl Error for FilterChainError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FilterChainError::Boxed(e) => Some(e.as_ref()),
            FilterChainError::AnyError(e) => Some(e.as_ref()),
        }
    }
}

impl FilterChainError {
    /// Checks if the error is of a specific type `T`.
    ///
    /// # Type Parameters
    /// * `T` - The error type to check against. Must implement `Error + Any + 'static`.
    ///
    /// # Returns
    /// * `true` if the error is of type `T`, `false` otherwise.
    ///
    /// # Examples
    /// ```
    /// use std::io;
    /// let err = FilterChainError::from(io::Error::new(io::ErrorKind::NotFound, "test"));
    /// assert!(err.is::<io::Error>());
    /// ```
    pub fn is<T: Error + Any + 'static>(&self) -> bool {
        match self {
            FilterChainError::AnyError(e) => (e as &dyn Any).is::<T>(),
            _ => false,
        }
    }

    /// Attempts to downcast the error to a concrete type `T`.
    ///
    /// Returns a reference to the downcasted type if successful, otherwise `None`.
    ///
    /// # Type Parameters
    /// * `T` - The target error type. Must implement `Error + Any + 'static`.
    ///
    /// # Examples
    /// ```
    /// use std::io;
    /// let err = FilterChainError::from(io::Error::new(io::ErrorKind::NotFound, "test"));
    /// if let Some(io_err) = err.downcast_ref::<io::Error>() {
    ///     println!("IO error: {}", io_err);
    /// }
    /// ```
    pub fn downcast_ref<T: Error + Any + 'static>(&self) -> Option<&T> {
        match self {
            FilterChainError::AnyError(e) => (e as &dyn Any).downcast_ref::<T>(),
            _ => None,
        }
    }
}

impl From<io::Error> for FilterError {
    fn from(e: io::Error) -> Self {
        FilterError::Io(e)
    }
}

impl From<Box<dyn ChainError>> for FilterError {
    fn from(error: Box<dyn ChainError>) -> Self {
        FilterError::Chain(FilterChainError::AnyError(error))
    }
}

impl From<Box<dyn Error + Send + Sync>> for FilterError {
    fn from(value: Box<dyn Error + Send + Sync>) -> Self {
        FilterError::Chain(FilterChainError::Boxed(value))
    }
}

pub trait ChainError: Error + Send + Sync + Any {}

impl<T: Error + Send + Sync + Any> ChainError for T {}
