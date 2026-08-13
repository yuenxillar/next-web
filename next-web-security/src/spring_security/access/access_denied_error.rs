use std::{borrow::Cow, fmt::Display};

use crate::authorization::AuthorizationResult;

/// If the authentication object does not have the required permissions, an error occurs
pub enum AccessDeniedError {
    /// An error occurs when invalid or missing CsrfToken is found in HttpRequest
    Csrf(String),
    /// An AccessDeniedError that contains the AuthorizationResult
    AuthorizationDenied {
        msg: String,
        result: Box<dyn AuthorizationResult>,
    },
    /// If the authorization request cannot be processed due to system issues, an error occurs.
    /// For example, if the AccessDecisionManager implementation cannot find the required method parameters, an error may occur.
    AuthorizationService(String),
    /// An error occurs when the expected CsrfToken exists but does not match the value on HttpRequest
    InvalidCsrfToken {
        expected_access_token: (String, String),
        actual_access_token: Option<String>,
    },
    /// An error occurred when the required CsrfToken was not found
    MissingCsrfToken,
}

impl AccessDeniedError {
    pub fn as_str<'a>(&'a self) -> Cow<'a, str> {
        match self {
            Self::Csrf(msg) => Cow::Borrowed(msg.as_str()),
            Self::AuthorizationDenied { msg, .. } => Cow::Borrowed(msg.as_str()),
            Self::AuthorizationService(msg) => Cow::Borrowed(msg.as_str()),
            Self::InvalidCsrfToken {
                expected_access_token,
                actual_access_token,
            } => {
                let param_name = &expected_access_token.0;
                let header_name = &expected_access_token.1;

                Cow::Owned(format!(
                      "Invalid CSRF Token '{}' was found on the request parameter '{}' or header '{}'.",
                      actual_access_token
                          .as_ref()
                          .map(|token| token.as_str())
                          .unwrap_or_default(), param_name, header_name
                  ))
            }
            Self::MissingCsrfToken => Cow::Borrowed(
                "Could not verify the provided CSRF token because no token was found to compare.",
            ),
        }
    }
}

impl Display for AccessDeniedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for AccessDeniedError {
    fn from(value: &str) -> Self {
        Self::AuthorizationService(value.to_string())
    }
}

impl From<String> for AccessDeniedError {
    fn from(value: String) -> Self {
        Self::AuthorizationService(value)
    }
}
