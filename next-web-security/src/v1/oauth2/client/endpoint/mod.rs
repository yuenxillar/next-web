//! The messages sent to the token endpoint of a provider and the client that
//! sends them.

use next_web_core::async_trait;

use crate::{
    core::{AuthenticationError, AuthenticationErrorKind},
    oauth2::{ClientRegistration, OAuth2AccessTokenResponse, OAuth2AuthorizationExchange},
};

/// The request the authorization code of a grant is exchanged with.
#[derive(Clone)]
pub struct OAuth2AuthorizationCodeGrantRequest {
    client_registration: ClientRegistration,
    authorization_exchange: OAuth2AuthorizationExchange,
}

impl OAuth2AuthorizationCodeGrantRequest {
    /// Creates the request of an authorization code grant.
    ///
    /// # Arguments
    ///
    /// * `client_registration` - The client the code was issued to.
    /// * `authorization_exchange` - The authorization request and response of
    ///   the grant.
    pub fn new(
        client_registration: ClientRegistration,
        authorization_exchange: OAuth2AuthorizationExchange,
    ) -> Self {
        Self {
            client_registration,
            authorization_exchange,
        }
    }

    /// Returns the client the code was issued to.
    pub fn client_registration(&self) -> &ClientRegistration {
        &self.client_registration
    }

    /// Returns the authorization request and response of the grant.
    pub fn authorization_exchange(&self) -> &OAuth2AuthorizationExchange {
        &self.authorization_exchange
    }

    /// Returns the authorization code of the grant.
    pub fn authorization_code(&self) -> Option<&str> {
        self.authorization_exchange.authorization_response().code()
    }

    /// Returns the proof key of the authorization request, if the client used
    /// one.
    pub fn code_verifier(&self) -> Option<&str> {
        self.authorization_exchange
            .authorization_request()
            .code_verifier()
    }
}

/// Sends the request of a grant to the token endpoint of a provider.
#[async_trait]
pub trait OAuth2AccessTokenResponseClient<R>: Send + Sync {
    /// Exchanges the request for the response of the token endpoint.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the grant.
    async fn get_token_response(
        &self,
        request: &R,
    ) -> Result<OAuth2AccessTokenResponse, AuthenticationError>;
}

/// A [`OAuth2AccessTokenResponseClient`] that sends the request of an
/// authorization code grant to the token endpoint of the provider.
///
/// The client authenticates the way its registration says:
/// `client_secret_basic` sends the credentials with the `Authorization`
/// header, `client_secret_post` sends them as parameters of the form, and a
/// public client sends only its identifier. The proof key of the authorization
/// request is sent with the code when the client used one.
#[derive(Clone)]
pub struct RestClientAuthorizationCodeTokenResponseClient {
    http_client: reqwest::Client,
}

impl RestClientAuthorizationCodeTokenResponseClient {
    /// Creates the client of the token endpoint.
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
        }
    }

    /// Creates the client of the token endpoint with the HTTP client of the
    /// application.
    ///
    /// # Arguments
    ///
    /// * `http_client` - The HTTP client the requests are sent with.
    pub fn with_http_client(http_client: reqwest::Client) -> Self {
        Self { http_client }
    }

    /// Sends the request of a grant to the token endpoint of the provider.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the grant.
    async fn send(
        &self,
        request: &OAuth2AuthorizationCodeGrantRequest,
    ) -> Result<OAuth2AccessTokenResponse, AuthenticationError> {
        let client_registration = request.client_registration();
        let token_uri = client_registration.token_uri().ok_or_else(|| {
            AuthenticationError::with_kind(
                "The client registration has no token URI",
                AuthenticationErrorKind::BadCredentials,
            )
        })?;

        let token_request = TokenRequest::of(request)?;
        let mut builder = self
            .http_client
            .post(token_uri)
            .header("Accept", "application/json")
            .form(&token_request.parameters);
        if let Some(basic) = token_request.client_secret_basic.as_ref() {
            builder = builder.basic_auth(&basic.0, Some(&basic.1));
        }

        let response = builder.send().await.map_err(|error| {
            AuthenticationError::with_kind(
                format!("The token endpoint is not reached: {error}"),
                AuthenticationErrorKind::AuthenticationService,
            )
        })?;
        let status = response.status();
        let document = response
            .json::<serde_json::Value>()
            .await
            .map_err(|error| {
                AuthenticationError::with_kind(
                    format!("The response of the token endpoint is not JSON: {error}"),
                    AuthenticationErrorKind::AuthenticationService,
                )
            })?;

        if !status.is_success() {
            return Err(error_of(&document, status));
        }

        OAuth2AccessTokenResponse::from_json(&document).ok_or_else(|| {
            AuthenticationError::with_kind(
                "The response of the token endpoint has no access token",
                AuthenticationErrorKind::AuthenticationService,
            )
        })
    }
}

impl Default for RestClientAuthorizationCodeTokenResponseClient {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for RestClientAuthorizationCodeTokenResponseClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RestClientAuthorizationCodeTokenResponseClient")
            .finish()
    }
}

#[async_trait]
impl OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>
    for RestClientAuthorizationCodeTokenResponseClient
{
    async fn get_token_response(
        &self,
        request: &OAuth2AuthorizationCodeGrantRequest,
    ) -> Result<OAuth2AccessTokenResponse, AuthenticationError> {
        self.send(request).await
    }
}

/// The parameters and the credentials of a request of a token endpoint.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TokenRequest {
    parameters: Vec<(String, String)>,
    client_secret_basic: Option<(String, String)>,
}

impl TokenRequest {
    /// Builds the request of an authorization code grant.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the grant.
    pub fn of(request: &OAuth2AuthorizationCodeGrantRequest) -> Result<Self, AuthenticationError> {
        let client_registration = request.client_registration();
        let code = request.authorization_code().ok_or_else(|| {
            AuthenticationError::with_kind(
                "The authorization response has no code",
                AuthenticationErrorKind::BadCredentials,
            )
        })?;

        let mut token_request = Self {
            parameters: vec![
                ("grant_type".to_string(), "authorization_code".to_string()),
                ("code".to_string(), code.to_string()),
            ],
            client_secret_basic: None,
        };

        if let Some(redirect_uri) = client_registration.redirect_uri().or_else(|| {
            request
                .authorization_exchange()
                .authorization_request()
                .redirect_uri()
        }) {
            token_request
                .parameters
                .push(("redirect_uri".to_string(), redirect_uri.to_string()));
        }
        if let Some(code_verifier) = request.code_verifier() {
            token_request
                .parameters
                .push(("code_verifier".to_string(), code_verifier.to_string()));
        }

        match client_registration.client_authentication_method() {
            crate::oauth2::client::registration::ClientAuthenticationMethod::ClientSecretBasic => {
                let client_secret = client_registration.client_secret().ok_or_else(|| {
                    AuthenticationError::with_kind(
                        "The client registration has no client secret",
                        AuthenticationErrorKind::BadCredentials,
                    )
                })?;
                token_request.client_secret_basic = Some((
                    client_registration.client_id().to_string(),
                    client_secret.to_string(),
                ));
            }
            crate::oauth2::client::registration::ClientAuthenticationMethod::ClientSecretPost => {
                let client_secret = client_registration.client_secret().ok_or_else(|| {
                    AuthenticationError::with_kind(
                        "The client registration has no client secret",
                        AuthenticationErrorKind::BadCredentials,
                    )
                })?;
                token_request.parameters.push((
                    "client_id".to_string(),
                    client_registration.client_id().to_string(),
                ));
                token_request
                    .parameters
                    .push(("client_secret".to_string(), client_secret.to_string()));
            }
            crate::oauth2::client::registration::ClientAuthenticationMethod::None => {
                token_request.parameters.push((
                    "client_id".to_string(),
                    client_registration.client_id().to_string(),
                ));
            }
        }

        Ok(token_request)
    }

    /// Returns the parameters of the form of the request.
    pub fn parameters(&self) -> &[(String, String)] {
        &self.parameters
    }

    /// Returns the credentials of the `Authorization` header, if the client
    /// authenticates with it.
    pub fn client_secret_basic(&self) -> Option<(&str, &str)> {
        self.client_secret_basic
            .as_ref()
            .map(|(client_id, client_secret)| (client_id.as_str(), client_secret.as_str()))
    }
}

/// Returns the error the token endpoint reported.
///
/// # Arguments
///
/// * `document` - The JSON document of the response.
/// * `status` - The status of the response.
fn error_of(document: &serde_json::Value, status: reqwest::StatusCode) -> AuthenticationError {
    let error = document
        .get("error")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("invalid_grant");
    let description = document
        .get("error_description")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    AuthenticationError::with_kind(
        format!(
            "The token endpoint reported {error} ({status}){}",
            if description.is_empty() {
                String::new()
            } else {
                format!(": {description}")
            }
        ),
        AuthenticationErrorKind::BadCredentials,
    )
}

#[cfg(test)]
mod tests {
    use super::{OAuth2AuthorizationCodeGrantRequest, TokenRequest};
    use crate::oauth2::{
        client::registration::ClientAuthenticationMethod, ClientRegistration,
        OAuth2AuthorizationExchange, OAuth2AuthorizationRequest, OAuth2AuthorizationResponse,
    };

    fn grant_request(
        client_authentication_method: ClientAuthenticationMethod,
        client_secret: Option<&str>,
        code_verifier: Option<&str>,
    ) -> OAuth2AuthorizationCodeGrantRequest {
        let mut registration = ClientRegistration::new(
            "demo",
            "Demo Provider",
            "https://provider.example/authorize",
        )
        .set_client_id("demo-client")
        .set_token_uri("https://provider.example/token")
        .set_redirect_uri("https://client.example/callback")
        .set_client_authentication_method(client_authentication_method);
        if let Some(client_secret) = client_secret {
            registration = registration.set_client_secret(client_secret);
        }

        let mut authorization_request = OAuth2AuthorizationRequest::new(
            "demo",
            "https://provider.example/authorize",
            "state",
            None,
            vec![],
        );
        if let Some(code_verifier) = code_verifier {
            authorization_request = authorization_request.set_code_verifier(code_verifier);
        }
        let authorization_exchange = OAuth2AuthorizationExchange::new(
            authorization_request,
            OAuth2AuthorizationResponse::success("the-code", Some("state".to_string())),
        );

        OAuth2AuthorizationCodeGrantRequest::new(registration, authorization_exchange)
    }

    #[test]
    fn the_request_of_a_confidential_client_authenticates_with_the_header() {
        let request = grant_request(
            ClientAuthenticationMethod::ClientSecretBasic,
            Some("secret"),
            None,
        );

        let token_request = TokenRequest::of(&request).expect("the request is built");

        assert_eq!(
            token_request.client_secret_basic(),
            Some(("demo-client", "secret"))
        );
        assert!(token_request
            .parameters()
            .contains(&("grant_type".to_string(), "authorization_code".to_string())));
        assert!(token_request
            .parameters()
            .contains(&("code".to_string(), "the-code".to_string())));
        assert!(token_request.parameters().contains(&(
            "redirect_uri".to_string(),
            "https://client.example/callback".to_string()
        )));
    }

    #[test]
    fn the_request_of_a_client_that_posts_its_secret_sends_it_as_a_parameter() {
        let request = grant_request(
            ClientAuthenticationMethod::ClientSecretPost,
            Some("secret"),
            None,
        );

        let token_request = TokenRequest::of(&request).expect("the request is built");

        assert_eq!(token_request.client_secret_basic(), None);
        assert!(token_request
            .parameters()
            .contains(&("client_id".to_string(), "demo-client".to_string())));
        assert!(token_request
            .parameters()
            .contains(&("client_secret".to_string(), "secret".to_string())));
    }

    #[test]
    fn the_request_of_a_public_client_carries_its_proof_key() {
        let request = grant_request(ClientAuthenticationMethod::None, None, Some("the-verifier"));

        let token_request = TokenRequest::of(&request).expect("the request is built");

        assert_eq!(token_request.client_secret_basic(), None);
        assert!(token_request
            .parameters()
            .contains(&("client_id".to_string(), "demo-client".to_string())));
        assert!(token_request
            .parameters()
            .contains(&("code_verifier".to_string(), "the-verifier".to_string())));
    }

    #[test]
    fn a_grant_without_a_code_is_reported() {
        let mut request = grant_request(ClientAuthenticationMethod::None, None, None);
        let exchange = request.authorization_exchange().clone();
        request = OAuth2AuthorizationCodeGrantRequest::new(
            request.client_registration().clone(),
            OAuth2AuthorizationExchange::new(
                exchange.authorization_request().clone(),
                OAuth2AuthorizationResponse::error("access_denied", None, None),
            ),
        );

        assert!(TokenRequest::of(&request).is_err());
    }
}
