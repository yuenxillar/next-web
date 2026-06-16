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
    Chain(Box<dyn std::error::Error + Send + Sync>),

    /// An HTTP protocol error with a status code and descriptive message.
    Http {
        /// The HTTP status code.
        status: u16,
        /// A human-readable description of the error.
        message: String,
    },

    /// A filter explicitly rejected the request with a status code and message.
    /// This is analogous to `HttpServletResponse::sendError` in the servlet API.
    Rejection {
        /// The HTTP status code for the rejection.
        status: u16,
        /// A human-readable description of the rejection reason.
        message: String,
    },

    /// A general-purpose custom error with a descriptive message.
    Custom(String),
}

impl FilterError {
    /// Creates a new `FilterError::Http` with the given status code and message.
    ///
    /// # Arguments
    ///
    /// * `status` - The HTTP status code.
    /// * `message` - A human-readable description of the error.
    pub fn http(status: u16, message: impl Into<String>) -> Self {
        FilterError::Http {
            status,
            message: message.into(),
        }
    }

    /// Creates a new `FilterError::Rejection` with the given status code and message.
    ///
    /// This is analogous to `HttpServletResponse::sendError` in the servlet API.
    ///
    /// # Arguments
    ///
    /// * `status` - The HTTP status code for the rejection.
    /// * `message` - A human-readable description of the rejection reason.
    pub fn reject(status: u16, message: impl Into<String>) -> Self {
        FilterError::Rejection {
            status,
            message: message.into(),
        }
    }

    /// Creates a new `FilterError::Custom` with the given message.
    ///
    /// # Arguments
    ///
    /// * `message` - A human-readable description of the error.
    pub fn custom(message: impl Into<String>) -> Self {
        FilterError::Custom(message.into())
    }

    /// Returns the associated HTTP status code, if any.
    pub fn status_code(&self) -> Option<u16> {
        match self {
            FilterError::Http { status, .. } => Some(*status),
            FilterError::Rejection { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Returns `true` if this is a `FilterError::Rejection`.
    pub fn is_rejection(&self) -> bool {
        matches!(self, FilterError::Rejection { .. })
    }

    /// Returns `true` if this is a `FilterError::Http`.
    pub fn is_http(&self) -> bool {
        matches!(self, FilterError::Http { .. })
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
            FilterError::Http { status, message } => {
                write!(f, "HTTP error {status}: {message}")
            }
            FilterError::Rejection { status, message } => {
                write!(f, "rejected {status}: {message}")
            }
            FilterError::Custom(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for FilterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FilterError::Io(e) => Some(e),
            FilterError::Chain(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

// ---- From conversions ----

impl From<io::Error> for FilterError {
    fn from(e: io::Error) -> Self {
        FilterError::Io(e)
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for FilterError {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        FilterError::Chain(e)
    }
}

/// Convenience trait for converting types directly into `FilterError::Http`.
///
/// This allows functions to return errors with status codes using the `?` operator
/// on `Result<T, (u16, &str)>` values.
pub trait IntoHttpError {
    /// Converts this value into a `FilterError::Http`.
    fn into_http_error(self) -> FilterError;
}

impl<S: Into<String>> IntoHttpError for (u16, S) {
    fn into_http_error(self) -> FilterError {
        FilterError::http(self.0, self.1)
    }
}

/// Convenience trait for converting types directly into `FilterError::Rejection`.
///
/// This allows functions to reject requests with status codes using the `?` operator
/// on `Result<T, (u16, &str)>` values.
pub trait IntoRejection {
    /// Converts this value into a `FilterError::Rejection`.
    fn into_rejection(self) -> FilterError;
}

impl<S: Into<String>> IntoRejection for (u16, S) {
    fn into_rejection(self) -> FilterError {
        FilterError::reject(self.0, self.1)
    }
}
