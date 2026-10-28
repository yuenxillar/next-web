use std::error::Error;
use std::fmt;

/// Errors that can occur while initializing, running, or shutting down a
/// long-lived service.
///
/// Most variants carry a human-readable message. [`ServiceError::IoError`]
/// preserves the original [`std::io::Error`] and exposes it through
/// [`Error::source`], so callers can walk the error chain.
#[derive(Debug)]
pub enum ServiceError {
    /// The service failed to initialize (bad config, missing dependency, etc.).
    InitFailed(String),

    /// The service failed at runtime while running.
    RuntimeError(String),

    /// A start request was issued for a service that is already running.
    AlreadyRunning,

    /// A requested service does not exist in the manager.
    NotFound,

    /// Shutting the service down exceeded the configured timeout.
    ShutdownTimeout,

    /// An I/O error occurred. The original error is preserved as the source.
    IoError(std::io::Error),

    /// Any other error not covered by the variants above.
    Other(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceError::InitFailed(msg) => {
                write!(f, "Service initialization failed: {msg}")
            }
            ServiceError::RuntimeError(msg) => {
                write!(f, "Service runtime error: {msg}")
            }
            ServiceError::AlreadyRunning => f.write_str("Service already running"),
            ServiceError::NotFound => f.write_str("Service not found"),
            ServiceError::ShutdownTimeout => f.write_str("Shutdown timeout"),
            ServiceError::IoError(err) => write!(f, "IO error: {err}"),
            ServiceError::Other(msg) => write!(f, "Other error: {msg}"),
        }
    }
}

impl Error for ServiceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            // Preserve the original io::Error so callers can inspect
            // e.g. `err.kind()` via `source().downcast_ref::<io::Error>()`.
            ServiceError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ServiceError {
    fn from(err: std::io::Error) -> Self {
        ServiceError::IoError(err)
    }
}
