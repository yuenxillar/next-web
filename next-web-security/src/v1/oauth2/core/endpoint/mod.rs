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
    code_verifier: Option<String>,
    nonce: Option<String>,
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
            code_verifier: None,
            nonce: None,
        }
    }

    /// Builder-style setter for the authorization grant type.
    /// Defaults to `AuthorizationCode`, mirroring the Java builder.
    pub fn set_grant_type(mut self, grant_type: AuthorizationGrantType) -> Self {
        self.grant_type = grant_type;
        self
    }

    /// Sets the proof key of the client.
    ///
    /// The verifier is kept with the authorization request, and the token
    /// endpoint of the provider is asked with it when the authorization code is
    /// exchanged.
    ///
    /// # Arguments
    ///
    /// * `code_verifier` - The proof key of the client.
    pub fn set_code_verifier(mut self, code_verifier: impl Into<String>) -> Self {
        self.code_verifier = Some(code_verifier.into());
        self
    }

    /// Returns the proof key of the client, if one is used.
    pub fn code_verifier(&self) -> Option<&str> {
        self.code_verifier.as_deref()
    }

    /// Sets the nonce of the authorization request.
    ///
    /// An OpenID Connect client sends a nonce, and the identifier token the
    /// grant receives has to carry it back.
    ///
    /// # Arguments
    ///
    /// * `nonce` - The nonce of the request.
    pub fn set_nonce(mut self, nonce: impl Into<String>) -> Self {
        self.nonce = Some(nonce.into());
        self
    }

    /// Returns the nonce of the request, if one is used.
    pub fn nonce(&self) -> Option<&str> {
        self.nonce.as_deref()
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

/// The response of the token endpoint of a provider.
///
/// The response carries the access token of the grant and, when the provider
/// issued one, the refresh token, the scopes, the type of the token, and the
/// lifetime of the access token.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OAuth2AccessTokenResponse {
    access_token: String,
    token_type: Option<String>,
    expires_in: Option<u64>,
    refresh_token: Option<String>,
    scopes: Vec<String>,
    id_token: Option<String>,
}

impl OAuth2AccessTokenResponse {
    /// Creates a response with an access token.
    ///
    /// # Arguments
    ///
    /// * `access_token` - The access token of the grant.
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            ..Self::default()
        }
    }

    /// Sets the type of the access token.
    ///
    /// # Arguments
    ///
    /// * `token_type` - The type of the token, such as `Bearer`.
    pub fn set_token_type(mut self, token_type: impl Into<String>) -> Self {
        self.token_type = Some(token_type.into());
        self
    }

    /// Sets the lifetime of the access token, in seconds.
    ///
    /// # Arguments
    ///
    /// * `expires_in` - The lifetime of the token, in seconds.
    pub fn set_expires_in(mut self, expires_in: u64) -> Self {
        self.expires_in = Some(expires_in);
        self
    }

    /// Sets the refresh token of the grant.
    ///
    /// # Arguments
    ///
    /// * `refresh_token` - The refresh token of the grant.
    pub fn set_refresh_token(mut self, refresh_token: impl Into<String>) -> Self {
        self.refresh_token = Some(refresh_token.into());
        self
    }

    /// Sets the scopes of the grant.
    ///
    /// # Arguments
    ///
    /// * `scopes` - The scopes of the grant.
    pub fn set_scopes<I, S>(mut self, scopes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.scopes = scopes.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the identifier token of the grant.
    ///
    /// # Arguments
    ///
    /// * `id_token` - The identifier token of the grant.
    pub fn set_id_token(mut self, id_token: impl Into<String>) -> Self {
        self.id_token = Some(id_token.into());
        self
    }

    /// Returns the access token of the grant.
    pub fn access_token(&self) -> &str {
        &self.access_token
    }

    /// Returns the type of the access token, if the provider named one.
    pub fn token_type(&self) -> Option<&str> {
        self.token_type.as_deref()
    }

    /// Returns the lifetime of the access token, if the provider named one.
    pub fn expires_in(&self) -> Option<u64> {
        self.expires_in
    }

    /// Returns the refresh token of the grant, if the provider issued one.
    pub fn refresh_token(&self) -> Option<&str> {
        self.refresh_token.as_deref()
    }

    /// Returns the scopes of the grant.
    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }

    /// Returns the identifier token of the grant, if the provider issued one.
    pub fn id_token(&self) -> Option<&str> {
        self.id_token.as_deref()
    }

    /// Reads a response from the JSON document of a token endpoint.
    ///
    /// # Arguments
    ///
    /// * `document` - The JSON document of the response.
    pub fn from_json(document: &serde_json::Value) -> Option<Self> {
        let access_token = document.get("access_token")?.as_str()?.to_string();
        let mut response = Self::new(access_token);
        if let Some(token_type) = document
            .get("token_type")
            .and_then(serde_json::Value::as_str)
        {
            response = response.set_token_type(token_type);
        }
        if let Some(expires_in) = document
            .get("expires_in")
            .and_then(serde_json::Value::as_u64)
        {
            response = response.set_expires_in(expires_in);
        }
        if let Some(refresh_token) = document
            .get("refresh_token")
            .and_then(serde_json::Value::as_str)
        {
            response = response.set_refresh_token(refresh_token);
        }
        if let Some(scopes) = document.get("scope").and_then(serde_json::Value::as_str) {
            response = response.set_scopes(scopes.split_whitespace());
        }
        if let Some(id_token) = document.get("id_token").and_then(serde_json::Value::as_str) {
            response = response.set_id_token(id_token);
        }

        Some(response)
    }
}

#[cfg(test)]
mod token_response_tests {
    use super::OAuth2AccessTokenResponse;

    #[test]
    fn a_response_is_read_from_the_json_document_of_a_token_endpoint() {
        let document = serde_json::json!({
            "access_token": "the-token",
            "token_type": "Bearer",
            "expires_in": 300,
            "refresh_token": "the-refresh-token",
            "scope": "read write",
        });

        let response =
            OAuth2AccessTokenResponse::from_json(&document).expect("the document is a response");

        assert_eq!(response.access_token(), "the-token");
        assert_eq!(response.token_type(), Some("Bearer"));
        assert_eq!(response.expires_in(), Some(300));
        assert_eq!(response.refresh_token(), Some("the-refresh-token"));
        assert_eq!(response.scopes(), ["read", "write"]);
    }

    #[test]
    fn a_document_without_an_access_token_is_not_a_response() {
        let document = serde_json::json!({ "error": "invalid_grant" });

        assert!(OAuth2AccessTokenResponse::from_json(&document).is_none());
    }
}
