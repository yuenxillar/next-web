//! The error of an authorization server endpoint.

use serde_json::{json, Value};

/// The error an endpoint of the authorization server reports.
///
/// The error carries the code and the description of the specification, which
/// is what the error response of an endpoint is built from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OAuth2Error {
    error: String,
    error_description: Option<String>,
    error_uri: Option<String>,
}

impl OAuth2Error {
    /// Creates an error.
    ///
    /// # Arguments
    ///
    /// * `error` - The error code of the specification.
    /// * `error_description` - The description of the error.
    pub fn new(error: impl Into<String>, error_description: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            error_description: Some(error_description.into()),
            error_uri: None,
        }
    }

    /// Creates an error with the given code and no description.
    ///
    /// # Arguments
    ///
    /// * `error` - The error code of the specification.
    pub fn with_code(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            error_description: None,
            error_uri: None,
        }
    }

    /// Returns the error code.
    pub fn error(&self) -> &str {
        &self.error
    }

    /// Returns the description of the error, if one is set.
    pub fn error_description(&self) -> Option<&str> {
        self.error_description.as_deref()
    }

    /// Returns the URI of the error, if one is set.
    pub fn error_uri(&self) -> Option<&str> {
        self.error_uri.as_deref()
    }

    /// Sets the URI of the error.
    ///
    /// # Arguments
    ///
    /// * `error_uri` - The URI of the error.
    pub fn set_error_uri(&mut self, error_uri: impl Into<String>) {
        self.error_uri = Some(error_uri.into());
    }

    /// Returns the error as the JSON body of an error response.
    pub fn to_json(&self) -> Value {
        let mut body = json!({ "error": self.error });
        if let Some(description) = self.error_description.as_ref() {
            body["error_description"] = json!(description);
        }
        if let Some(uri) = self.error_uri.as_ref() {
            body["error_uri"] = json!(uri);
        }
        body
    }

    /// Returns `invalid_request`.
    ///
    /// # Arguments
    ///
    /// * `description` - The description of the error.
    pub fn invalid_request(description: impl Into<String>) -> Self {
        Self::new("invalid_request", description)
    }

    /// Returns `invalid_client`.
    ///
    /// # Arguments
    ///
    /// * `description` - The description of the error.
    pub fn invalid_client(description: impl Into<String>) -> Self {
        Self::new("invalid_client", description)
    }

    /// Returns `invalid_grant`.
    ///
    /// # Arguments
    ///
    /// * `description` - The description of the error.
    pub fn invalid_grant(description: impl Into<String>) -> Self {
        Self::new("invalid_grant", description)
    }

    /// Returns `invalid_token`.
    ///
    /// # Arguments
    ///
    /// * `description` - The description of the error.
    pub fn invalid_token(description: impl Into<String>) -> Self {
        Self::new("invalid_token", description)
    }

    /// Returns `unauthorized_client`.
    ///
    /// # Arguments
    ///
    /// * `description` - The description of the error.
    pub fn unauthorized_client(description: impl Into<String>) -> Self {
        Self::new("unauthorized_client", description)
    }

    /// Returns `unsupported_grant_type`.
    ///
    /// # Arguments
    ///
    /// * `description` - The description of the error.
    pub fn unsupported_grant_type(description: impl Into<String>) -> Self {
        Self::new("unsupported_grant_type", description)
    }

    /// Returns `unsupported_response_type`.
    ///
    /// # Arguments
    ///
    /// * `description` - The description of the error.
    pub fn unsupported_response_type(description: impl Into<String>) -> Self {
        Self::new("unsupported_response_type", description)
    }

    /// Returns `invalid_scope`.
    ///
    /// # Arguments
    ///
    /// * `description` - The description of the error.
    pub fn invalid_scope(description: impl Into<String>) -> Self {
        Self::new("invalid_scope", description)
    }

    /// Returns `access_denied`.
    ///
    /// # Arguments
    ///
    /// * `description` - The description of the error.
    pub fn access_denied(description: impl Into<String>) -> Self {
        Self::new("access_denied", description)
    }

    /// Returns `server_error`.
    ///
    /// # Arguments
    ///
    /// * `description` - The description of the error.
    pub fn server_error(description: impl Into<String>) -> Self {
        Self::new("server_error", description)
    }

    /// Returns the value of the `WWW-Authenticate` header of the error.
    ///
    /// The header is what a client authentication failure reports.
    pub fn www_authenticate(&self) -> String {
        match self.error_description.as_ref() {
            Some(description) => format!(
                "Bearer error=\"{}\", error_description=\"{}\"",
                self.error, description
            ),
            None => format!("Bearer error=\"{}\"", self.error),
        }
    }
}

impl std::fmt::Display for OAuth2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.error_description.as_ref() {
            Some(description) => write!(f, "{}: {}", self.error, description),
            None => write!(f, "{}", self.error),
        }
    }
}

impl std::error::Error for OAuth2Error {}

#[cfg(test)]
mod tests {
    use super::OAuth2Error;

    #[test]
    fn an_error_carries_its_code_and_description() {
        let error = OAuth2Error::invalid_grant("The authorization code is invalid");

        assert_eq!(error.error(), "invalid_grant");
        assert_eq!(
            error.error_description(),
            Some("The authorization code is invalid")
        );
        assert_eq!(
            error.to_json(),
            serde_json::json!({
                "error": "invalid_grant",
                "error_description": "The authorization code is invalid",
            })
        );
    }

    #[test]
    fn an_error_without_a_description_has_no_description() {
        let error = OAuth2Error::with_code("invalid_client");

        assert_eq!(
            error.to_json(),
            serde_json::json!({ "error": "invalid_client" })
        );
        assert_eq!(error.www_authenticate(), "Bearer error=\"invalid_client\"");
    }
}
