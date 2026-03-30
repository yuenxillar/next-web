use next_web_core::error::BoxError;
use thiserror::Error;

/// Error returned by the mail module.
#[derive(Debug, Error)]
pub enum MailError {
    #[error("mail parse error: {0}")]
    ParseError(String),
    #[error("mail send error: {0}")]
    SendError(String),
    #[error("mail authentication error: {0}")]
    AuthenticationError(String),
    #[error("{message}")]
    OperationError {
        message: String,
        #[source]
        source: Option<BoxError>,
    },
}

impl MailError {
    /// Build an operation error that keeps the original source.
    pub fn with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::OperationError {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Build an operation error without an underlying source.
    pub fn operation(message: impl Into<String>) -> Self {
        Self::OperationError {
            message: message.into(),
            source: None,
        }
    }
}
