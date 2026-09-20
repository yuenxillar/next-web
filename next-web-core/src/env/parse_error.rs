use std::fmt;

/// An error produced while parsing a profile expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// The offending expression.
    pub expression: Option<String>,
    /// A human-readable description of the problem.
    pub message: String,
}

impl ParseError {
    pub fn new(expression: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            expression: Some(expression.into()),
            message: message.into(),
        }
    }

    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            expression: None,
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid profile expression `{:?}`: {}",
            self.expression, self.message
        )
    }
}

impl std::error::Error for ParseError {}
