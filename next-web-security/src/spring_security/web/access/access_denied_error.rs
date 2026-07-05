use core::fmt;
use std::error::Error;

use crate::authorization::AuthorizationDeniedError;

#[derive(Debug, Clone)]
pub enum AccessDeniedError {
    AuthorizationDeniedError(AuthorizationDeniedError),
    AuthorizationServiceError(String),
    CsrfError(String),
    InvalidCsrfToken {
        expected_access_token: (String, String),
        actual_access_token: Option<String>,
    },
    MissingCsrfToken {
        actual_token: Option<String>,
    },
}

impl AccessDeniedError {
    pub fn authorization_denied_error(&self) -> Option<&AuthorizationDeniedError> {
        if let AccessDeniedError::AuthorizationDeniedError(err) = self {
            Some(err)
        } else {
            None
        }
    }

    pub fn msg(&self) -> &str {
        match self {
            AccessDeniedError::AuthorizationDeniedError(err) => err.msg(),
            AccessDeniedError::AuthorizationServiceError(msg) => msg.as_str(),
            AccessDeniedError::CsrfError(msg) => msg.as_str(),
            AccessDeniedError::InvalidCsrfToken { .. } => "Invalid CSRF token",
            AccessDeniedError::MissingCsrfToken { .. } => {
                "Could not verify the provided CSRF token because no token was found to compare."
            }
        }
    }
}

impl fmt::Display for AccessDeniedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthorizationDeniedError(err) => {
                write!(f, "Authorization denied: {}", err.msg())
            }
            Self::AuthorizationServiceError(msg) => {
                write!(f, "Authorization service error: {}", msg)
            }
            Self::CsrfError(msg) => write!(f, "CSRF error: {}", msg),
            Self::InvalidCsrfToken {
                expected_access_token,
                actual_access_token,
            } => {
                write!(
                    f,
                    "Invalid CSRF token: '{:?}' was found on the request parameter '{}', or header  '{}'.",
                    actual_access_token, expected_access_token.0.as_str(), expected_access_token.1.as_str()
                )
            }
            #[allow(unused)]
            Self::MissingCsrfToken { actual_token } => {
                write!(f, "Could not verify the provided CSRF token because no token was found to compare.")
            }
        }
    }
}

impl Error for AccessDeniedError {}
