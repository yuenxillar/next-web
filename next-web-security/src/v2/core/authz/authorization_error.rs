use std::{error::Error, fmt::Display};

/// The reason a subject was not allowed to do what it asked for.
///
/// Shiro separates the two cases an application answers differently: the subject
/// is not authenticated at all, or it is authenticated but not allowed.
#[derive(Debug, PartialEq, Eq)]
pub enum AuthorizationError {
    /// The subject is not authenticated, so it cannot be authorized.
    Unauthorized(String),
    /// The subject is authenticated but does not hold the right it asked for.
    Forbidden(String),
}

impl AuthorizationError {
    /// Creates the error of a subject that is not authenticated.
    ///
    /// # Arguments
    ///
    /// * `message` - What the subject was not allowed to do.
    pub fn unauthorized(message: impl ToString) -> Self {
        Self::Unauthorized(message.to_string())
    }

    /// Creates the error of a subject that lacks the right it asked for.
    ///
    /// # Arguments
    ///
    /// * `message` - What the subject was not allowed to do.
    pub fn forbidden(message: impl ToString) -> Self {
        Self::Forbidden(message.to_string())
    }

    /// Returns the message of the error.
    pub fn message(&self) -> &str {
        match self {
            Self::Unauthorized(message) | Self::Forbidden(message) => message,
        }
    }
}

impl Error for AuthorizationError {}

impl Display for AuthorizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthorizationError::Unauthorized(message) => {
                write!(f, "The subject is not authenticated: {message}")
            }
            AuthorizationError::Forbidden(message) => {
                write!(f, "The subject is not permitted: {message}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AuthorizationError;

    #[test]
    fn the_two_cases_display_differently() {
        let unauthorized = AuthorizationError::unauthorized("the session expired");
        let forbidden = AuthorizationError::forbidden("printer:print");

        assert_eq!(
            unauthorized.to_string(),
            "The subject is not authenticated: the session expired"
        );
        assert_eq!(
            forbidden.to_string(),
            "The subject is not permitted: printer:print"
        );
        assert_eq!(forbidden.message(), "printer:print");
        assert_ne!(unauthorized, forbidden);
    }
}
