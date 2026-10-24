use std::error::Error;
use std::fmt;

/// Errors that can occur while performing idempotent operations.
///
/// All variants are self-contained: none wrap an underlying error, so
/// [`Error::source`] always returns `None` and the error chain has depth one.
#[derive(Debug)]
pub enum IdempotencyError {
    /// The backing storage failed to read or write.
    StorageError(String),

    /// A value could not be serialized before being stored.
    SerializationError(String),

    /// A stored value could not be deserialized on retrieval.
    DeserializationError(String),

    /// The idempotency key already exists, so the operation was rejected.
    KeyExists,
}

impl fmt::Display for IdempotencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdempotencyError::StorageError(msg) => {
                write!(f, "Storage error: {msg}")
            }
            IdempotencyError::SerializationError(msg) => {
                write!(f, "Serialization error: {msg}")
            }
            IdempotencyError::DeserializationError(msg) => {
                write!(f, "Deserialization error: {msg}")
            }
            IdempotencyError::KeyExists => f.write_str("Key already exists"),
        }
    }
}

impl Error for IdempotencyError {}
