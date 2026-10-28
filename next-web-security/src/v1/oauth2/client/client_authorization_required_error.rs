//! The error reported when a client has to obtain the authorization of the
//! resource owner first.

use std::fmt;

use next_web_core::http::filter::{FilterChainError, FilterError};

/// Indicates that an OAuth 2.0 Client is required to obtain authorization
/// from the Resource Owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientAuthorizationRequiredError {
    client_registration_id: String,
}

impl ClientAuthorizationRequiredError {
    pub fn new(client_registration_id: impl Into<String>) -> Self {
        Self {
            client_registration_id: client_registration_id.into(),
        }
    }

    pub fn client_registration_id(&self) -> &str {
        &self.client_registration_id
    }
}

impl fmt::Display for ClientAuthorizationRequiredError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Authorization required for Client Registration Id: {}",
            self.client_registration_id
        )
    }
}

impl std::error::Error for ClientAuthorizationRequiredError {}

impl From<ClientAuthorizationRequiredError> for FilterError {
    fn from(error: ClientAuthorizationRequiredError) -> Self {
        FilterError::Chain(FilterChainError::AnyError(Box::new(error)))
    }
}
