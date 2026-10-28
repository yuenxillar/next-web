//! Tests of the OAuth 2.0 client configuration.
//!
//! A client configuration is only usable when the two halves of the
//! authorization code grant work: the request that starts the grant is answered
//! with a redirect to the provider, and the callback that carries the code is
//! exchanged for a token, which the application then holds for the user. The
//! tests run both requests through the chain the configuration builds.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{body::Body, extract::Request};
use next_web_core::{async_trait, http::HttpRequest as _};

use super::*;
use crate::{
    config::{
        authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
        security_builder::SecurityBuilder,
        web::builders::HttpSecurity,
    },
    core::AuthenticationError,
    oauth2::{
        AuthorizationGrantType, AuthorizationRequestRepository, ClientRegistration,
        ClientRegistrationRepository, InMemoryClientRegistrationRepository,
        OAuth2AccessTokenResponse, OAuth2AccessTokenResponseClient,
        OAuth2AuthorizationCodeGrantRequest, OAuth2AuthorizationRequest, OAuth2AuthorizedClient,
        OAuth2AuthorizedClientRepository,
    },
    web::{security_filter_chain::SecurityFilterChain, test_support::Chain},
};

/// The registration the tests configure.
const REGISTRATION_ID: &str = "test";
/// The URI the provider sends the client back to.
const REDIRECT_URI: &str = "http://localhost/authorized";

/// Returns the registration of the provider the tests use.
fn registration() -> ClientRegistration {
    ClientRegistration::new(
        REGISTRATION_ID,
        "Test Client",
        "https://provider.example/authorize",
    )
    .set_client_id("test-client")
    .set_client_secret("test-secret")
    .set_redirect_uri(REDIRECT_URI)
    .set_token_uri("https://provider.example/token")
    .set_scopes(["read"])
    .set_authorization_grant_type(AuthorizationGrantType::AuthorizationCode)
}

/// Keeps the authorization request between the two halves of the grant.
#[derive(Clone, Default)]
struct RequestStore(Arc<Mutex<Option<OAuth2AuthorizationRequest>>>);

impl AuthorizationRequestRepository for RequestStore {
    fn load_authorization_request(
        &self,
        _request: &dyn next_web_core::http::HttpRequest,
    ) -> Option<OAuth2AuthorizationRequest> {
        self.0.lock().ok()?.clone()
    }

    fn save_authorization_request(
        &self,
        authorization_request: Option<OAuth2AuthorizationRequest>,
        _request: &mut dyn next_web_core::http::HttpRequest,
        _response: &mut dyn next_web_core::http::HttpResponse,
    ) {
        if let Ok(mut slot) = self.0.lock() {
            *slot = authorization_request;
        }
    }

    fn remove_authorization_request(
        &self,
        _request: &mut dyn next_web_core::http::HttpRequest,
    ) -> Option<OAuth2AuthorizationRequest> {
        self.0.lock().ok()?.take()
    }
}

/// Holds the authorized clients the chain obtains.
#[derive(Clone, Default)]
struct ClientStore(Arc<Mutex<HashMap<String, OAuth2AuthorizedClient>>>);

impl ClientStore {
    /// Returns the client of the given registration, if one was stored.
    fn client(&self, registration_id: &str) -> Option<OAuth2AuthorizedClient> {
        self.0.lock().ok()?.get(registration_id).cloned()
    }
}

impl OAuth2AuthorizedClientRepository for ClientStore {
    fn save_authorized_client(
        &self,
        authorized_client: OAuth2AuthorizedClient,
        _principal: &dyn crate::core::Authentication,
        _request: &dyn next_web_core::http::HttpRequest,
        _response: &mut dyn next_web_core::http::HttpResponse,
    ) {
        if let Ok(mut clients) = self.0.lock() {
            clients.insert(
                authorized_client
                    .client_registration()
                    .registration_id()
                    .to_string(),
                authorized_client,
            );
        }
    }

    fn load_authorized_client(
        &self,
        client_registration_id: &str,
        _principal: &dyn crate::core::Authentication,
        _request: &dyn next_web_core::http::HttpRequest,
    ) -> Option<OAuth2AuthorizedClient> {
        self.client(client_registration_id)
    }
}

/// Answers the token endpoint with a fixed token, instead of calling a provider.
struct TokenClient;

#[async_trait]
impl OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest> for TokenClient {
    async fn get_token_response(
        &self,
        _request: &OAuth2AuthorizationCodeGrantRequest,
    ) -> Result<OAuth2AccessTokenResponse, AuthenticationError> {
        Ok(OAuth2AccessTokenResponse::new("access-token")
            .set_refresh_token("refresh-token")
            .set_scopes(["read"]))
    }
}

/// Returns an HTTP security that drives the authorization code grant of the
/// provider the tests use.
fn http_security(requests: RequestStore, clients: ClientStore) -> HttpSecurity {
    let clients = Arc::new(clients);
    let requests = Arc::new(requests);
    let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), HashMap::new());
    http.oauth2_client(|oauth2_client| {
        oauth2_client
            .client_registration_repository(Arc::new(InMemoryClientRegistrationRepository::new([
                registration(),
            ])))
            .authorized_client_repository(clients.clone())
            .authorization_code_grant(|grant| {
                grant
                    .authorization_request_repository(requests.clone())
                    .access_token_response_client(Arc::new(TokenClient));
            });
    });

    http
}

/// Returns a `GET` of the given URL.
fn request(uri: &str) -> Request {
    Request::builder()
        .uri(uri)
        .body(Body::empty())
        .expect("a valid request")
}

/// Runs one request through the chain, and returns the status and the location
/// of the answer.
async fn run(mut http: HttpSecurity, uri: &str) -> (usize, Option<String>) {
    let chain = http.build();
    let filters = Arc::new(chain.get_filters().to_vec());
    let mut request = request(uri);
    let mut response = axum::response::Response::default();
    Chain::new(filters)
        .run(&mut request, &mut response)
        .await
        .expect("the chain runs");
    let location = response
        .headers()
        .get("location")
        .and_then(|location| location.to_str().ok())
        .map(|location| location.to_string());

    (response.status().as_u16() as usize, location)
}

/// Returns the state of the authorization request in the given location.
fn state_of(location: &str) -> String {
    location
        .split(['?', '&'])
        .find_map(|parameter| parameter.strip_prefix("state="))
        .expect("the redirect carries the state of the authorization request")
        .to_string()
}

#[test]
fn the_configuration_shares_the_repositories() {
    let http = {
        let mut http = http_security(RequestStore::default(), ClientStore::default());
        http.build();

        http
    };

    assert!(
        http.shared_object::<Arc<dyn ClientRegistrationRepository>>()
            .is_some(),
        "the registrations of the clients are shared"
    );
    assert!(
        http.shared_object::<Arc<dyn OAuth2AuthorizedClientRepository>>()
            .is_some(),
        "the authorized clients are shared"
    );
}

#[tokio::test]
async fn a_request_for_the_authorization_endpoint_is_redirected_to_the_provider() {
    let (status, location) = run(
        http_security(RequestStore::default(), ClientStore::default()),
        &format!("/oauth2/authorization/{REGISTRATION_ID}"),
    )
    .await;

    let location = location.expect("the request is answered with a redirect");
    assert_eq!(status, 302, "the request is answered with a redirect");
    assert!(
        location.starts_with("https://provider.example/authorize?"),
        "the redirect goes to the authorization endpoint of the provider, it was: {location}"
    );
    assert!(
        location.contains("client_id=test-client"),
        "the redirect names the client, it was: {location}"
    );
    assert!(
        location.contains("response_type=code"),
        "the redirect asks for a code, it was: {location}"
    );
    assert!(
        location.contains("state="),
        "the redirect carries a state, it was: {location}"
    );
}

#[tokio::test]
async fn the_callback_exchanges_the_code_and_stores_the_authorized_client() {
    let requests = RequestStore::default();
    let clients = ClientStore::default();

    // The first half: the client is sent to the provider.
    let (status, location) = run(
        http_security(requests.clone(), clients.clone()),
        &format!("/oauth2/authorization/{REGISTRATION_ID}"),
    )
    .await;
    assert_eq!(status, 302, "the request is answered with a redirect");
    let state = state_of(&location.expect("the request is answered with a redirect"));

    // The second half: the provider sends the client back with a code.
    let (status, location) = run(
        http_security(requests.clone(), clients.clone()),
        &format!("/authorized?code=the-code&state={state}"),
    )
    .await;

    assert_eq!(
        status, 302,
        "the callback is answered with a redirect, it redirected to: {location:?}"
    );
    let client = clients
        .client(REGISTRATION_ID)
        .expect("the token of the grant is stored for the client");
    assert_eq!(
        client.access_token().token_value(),
        "access-token",
        "the stored client holds the token of the exchange"
    );
    assert_eq!(
        client.refresh_token().map(|token| token.value()),
        Some("refresh-token"),
        "the stored client holds the refresh token of the exchange"
    );
}

#[tokio::test]
async fn a_callback_without_a_code_is_not_matched_by_the_grant() {
    let requests = RequestStore::default();
    let clients = ClientStore::default();
    let (status, _location) = run(
        http_security(requests, clients.clone()),
        "/authorized?state=unknown",
    )
    .await;

    assert_eq!(
        status, 200,
        "the chain leaves a request that carries no code to the application"
    );
    assert!(
        clients.client(REGISTRATION_ID).is_none(),
        "no client is stored for a request that is not a callback"
    );
}
