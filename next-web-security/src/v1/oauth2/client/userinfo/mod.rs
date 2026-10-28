//! The user information loaded from the user-info endpoint.

use std::collections::HashMap;

use next_web_core::async_trait;
use serde_json::Value;

use crate::{
    core::{AuthenticationError, AuthenticationErrorKind},
    oauth2::{ClientRegistration, OAuth2AccessTokenResponse, OAuth2User},
};

/// The request the user information of a grant is loaded with.
#[derive(Clone)]
pub struct OAuth2UserRequest {
    client_registration: ClientRegistration,
    access_token_response: OAuth2AccessTokenResponse,
}

impl OAuth2UserRequest {
    /// Creates the request of a grant.
    ///
    /// # Arguments
    ///
    /// * `client_registration` - The client the grant was issued to.
    /// * `access_token_response` - The response of the token endpoint.
    pub fn new(
        client_registration: ClientRegistration,
        access_token_response: OAuth2AccessTokenResponse,
    ) -> Self {
        Self {
            client_registration,
            access_token_response,
        }
    }

    /// Returns the client the grant was issued to.
    pub fn client_registration(&self) -> &ClientRegistration {
        &self.client_registration
    }

    /// Returns the response of the token endpoint.
    pub fn access_token_response(&self) -> &OAuth2AccessTokenResponse {
        &self.access_token_response
    }
}

/// Loads the user information of a grant.
#[async_trait]
pub trait OAuth2UserService<R, U>: Send + Sync {
    /// Loads the user the grant was issued for.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the grant.
    async fn load_user(&self, request: &R) -> Result<U, AuthenticationError>;
}

/// A [`OAuth2UserService`] that reads the user from the user-info endpoint of
/// the provider.
///
/// The endpoint is asked with the access token of the grant, and the JSON
/// document it answers with is the source of the name and of the attributes of
/// the user. The registration of the client names the endpoint, so a provider
/// that does not serve one cannot be logged in to this way.
#[derive(Clone)]
pub struct DefaultOAuth2UserService {
    http_client: reqwest::Client,
}

impl DefaultOAuth2UserService {
    /// Creates the service with the HTTP client of the application.
    ///
    /// # Arguments
    ///
    /// * `http_client` - The HTTP client the requests are sent with.
    pub fn with_http_client(http_client: reqwest::Client) -> Self {
        Self { http_client }
    }
}

impl Default for DefaultOAuth2UserService {
    fn default() -> Self {
        Self {
            http_client: reqwest::Client::new(),
        }
    }
}

impl std::fmt::Debug for DefaultOAuth2UserService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefaultOAuth2UserService").finish()
    }
}

#[async_trait]
impl OAuth2UserService<OAuth2UserRequest, OAuth2User> for DefaultOAuth2UserService {
    async fn load_user(
        &self,
        request: &OAuth2UserRequest,
    ) -> Result<OAuth2User, AuthenticationError> {
        let access_token = request.access_token_response().access_token();
        if access_token.trim().is_empty() {
            return Err(AuthenticationError::with_kind(
                "The grant has no access token",
                AuthenticationErrorKind::BadCredentials,
            ));
        }

        let user_info_uri = request
            .client_registration()
            .user_info_uri()
            .ok_or_else(|| {
                AuthenticationError::with_kind(
                    "The client registration has no user info URI",
                    AuthenticationErrorKind::BadCredentials,
                )
            })?;

        let response = self
            .http_client
            .get(user_info_uri)
            .bearer_auth(access_token)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|error| {
                AuthenticationError::with_kind(
                    format!("The user info endpoint is not reached: {error}"),
                    AuthenticationErrorKind::AuthenticationService,
                )
            })?;
        let status = response.status();
        let document = response.json::<Value>().await.map_err(|error| {
            AuthenticationError::with_kind(
                format!("The response of the user info endpoint is not JSON: {error}"),
                AuthenticationErrorKind::AuthenticationService,
            )
        })?;

        if !status.is_success() {
            return Err(AuthenticationError::with_kind(
                format!("The user info endpoint reported {status}"),
                AuthenticationErrorKind::BadCredentials,
            ));
        }

        user_of(&document)
    }
}

/// Builds the user of a response of a user-info endpoint.
///
/// The name of the user is the `sub` claim, which is what the specification
/// names the user by, and every attribute of the document is kept.
///
/// # Arguments
///
/// * `document` - The JSON document of the endpoint.
pub fn user_of(document: &Value) -> Result<OAuth2User, AuthenticationError> {
    let name = document
        .get("sub")
        .and_then(Value::as_str)
        .or_else(|| document.get("name").and_then(Value::as_str))
        .ok_or_else(|| {
            AuthenticationError::with_kind(
                "The user info response has no sub claim",
                AuthenticationErrorKind::AuthenticationService,
            )
        })?;

    let attributes = document
        .as_object()
        .map(|object| {
            object
                .iter()
                .map(|(name, value)| {
                    let value = match value {
                        Value::String(value) => value.clone(),
                        value => value.to_string(),
                    };
                    (name.clone(), value)
                })
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();

    Ok(OAuth2User::new(name, attributes))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::user_of;

    #[test]
    fn a_user_is_read_from_the_document_of_the_endpoint() {
        let document = json!({
            "sub": "user",
            "name": "The User",
            "email": "user@example.com",
            "email_verified": true,
        });

        let user = user_of(&document).expect("the user is read");

        assert_eq!(user.name(), "user");
        assert_eq!(user.attributes().get("name"), Some(&"The User".to_string()));
        assert_eq!(
            user.attributes().get("email_verified"),
            Some(&"true".to_string())
        );
    }

    #[test]
    fn a_document_without_a_subject_is_reported() {
        let document = json!({ "name": "The User" });

        // The name of the document is used when the specification names no
        // subject, and a document without either is refused.
        assert_eq!(
            user_of(&document).expect("the name is used").name(),
            "The User"
        );
        assert!(user_of(&json!({})).is_err());
    }
}
