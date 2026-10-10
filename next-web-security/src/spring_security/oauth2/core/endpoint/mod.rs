//! The messages exchanged with the OAuth 2.0 authorization endpoint and the
//! OAuth 2.0 token endpoint.

use std::fmt;

use super::AuthorizationGrantType;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OAuth2AuthorizationRequest {
    registration_id: String,
    authorization_request_uri: String,
    state: String,
    redirect_uri: Option<String>,
    scopes: Vec<String>,
    grant_type: AuthorizationGrantType,
}

impl OAuth2AuthorizationRequest {
    pub fn new(
        registration_id: impl Into<String>,
        authorization_request_uri: impl Into<String>,
        state: impl Into<String>,
        redirect_uri: Option<String>,
        scopes: Vec<String>,
    ) -> Self {
        Self {
            registration_id: registration_id.into(),
            authorization_request_uri: authorization_request_uri.into(),
            state: state.into(),
            redirect_uri,
            scopes,
            grant_type: AuthorizationGrantType::AuthorizationCode,
        }
    }

    /// Builder-style setter for the authorization grant type.
    /// Defaults to `AuthorizationCode`, mirroring the Java builder.
    pub fn set_grant_type(mut self, grant_type: AuthorizationGrantType) -> Self {
        self.grant_type = grant_type;
        self
    }

    pub fn registration_id(&self) -> &str {
        &self.registration_id
    }

    pub fn authorization_request_uri(&self) -> &str {
        &self.authorization_request_uri
    }

    pub fn state(&self) -> &str {
        &self.state
    }

    pub fn redirect_uri(&self) -> Option<&str> {
        self.redirect_uri.as_deref()
    }

    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }

    pub fn grant_type(&self) -> &AuthorizationGrantType {
        &self.grant_type
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OAuth2AuthorizationResponse {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

impl OAuth2AuthorizationResponse {
    pub fn success(code: impl Into<String>, state: Option<String>) -> Self {
        Self {
            code: Some(code.into()),
            state,
            error: None,
            error_description: None,
        }
    }

    pub fn error(
        error: impl Into<String>,
        error_description: Option<String>,
        state: Option<String>,
    ) -> Self {
        Self {
            code: None,
            state,
            error: Some(error.into()),
            error_description,
        }
    }

    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    pub fn state(&self) -> Option<&str> {
        self.state.as_deref()
    }

    pub fn error_code(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn error_description(&self) -> Option<&str> {
        self.error_description.as_deref()
    }

    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OAuth2AuthorizationExchange {
    authorization_request: OAuth2AuthorizationRequest,
    authorization_response: OAuth2AuthorizationResponse,
}
impl fmt::Display for OAuth2AuthorizationExchange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OAuth2AuthorizationExchange(state={:?})",
            self.authorization_response.state()
        )
    }
}
impl crate::web::authentication::Identity for OAuth2AuthorizationExchange {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl OAuth2AuthorizationExchange {
    pub fn new(
        authorization_request: OAuth2AuthorizationRequest,
        authorization_response: OAuth2AuthorizationResponse,
    ) -> Self {
        Self {
            authorization_request,
            authorization_response,
        }
    }

    pub fn authorization_request(&self) -> &OAuth2AuthorizationRequest {
        &self.authorization_request
    }

    pub fn authorization_response(&self) -> &OAuth2AuthorizationResponse {
        &self.authorization_response
    }
}

#[derive(Clone, Debug, Default)]
pub struct OAuth2AccessTokenResponse {
    access_token: String,
}

impl OAuth2AccessTokenResponse {
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
        }
    }

    pub fn access_token(&self) -> &str {
        &self.access_token
    }
}
