use std::{fmt, sync::Arc};

use next_web_core::{
    async_trait,
    filter::FilterError,
    http::{HttpMethod, StatusCode},
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use crate::{
    authorization::AuthenticationManager,
    core::{
        context::{SecurityContextHolder, SecurityContextHolderStrategy},
        {Authentication, AuthenticationError, AuthenticationErrorKind},
    },
    oauth2::{
        request_parameter, AuthorizationRequestRepository, ClientRegistrationRepository,
        HttpSessionOAuth2AuthorizationRequestRepository,
        OAuth2AuthorizationCodeAuthenticationToken, OAuth2AuthorizationExchange,
        OAuth2AuthorizationResponse, OAuth2AuthorizedClient, OAuth2AuthorizedClientRepository,
    },
    web::{
        context::{RequestAttributeSecurityContextRepository, SecurityContextRepository},
        savedrequest::{HttpSessionRequestCache, RequestCache},
        DefaultRedirectStrategy, RedirectStrategy,
    },
};

/// The default base URI used to receive the Authorization Code Grant callback.
pub const DEFAULT_AUTHORIZATION_RESPONSE_BASE_URI: &'static str = "/login/oauth2/code";

/// Returns the path of `url`, which may be absolute.
///
/// The path of the redirect URI of a saved request is compared with the path of
/// a request, which carries no scheme and no authority.
///
/// # Arguments
///
/// * `url` - The URL the path is read from.
fn redirect_uri_path(url: &str) -> &str {
    let after_authority = match url.split_once("://") {
        Some((_, authority_and_path)) => authority_and_path,
        None => url,
    };
    let path = match after_authority.find('/') {
        Some(index) => &after_authority[index..],
        None => "/",
    };

    match path.split_once(['?', '#']) {
        Some((path, _)) => path,
        None => path,
    }
}

/// A filter that completes the OAuth 2.0 Authorization Code Grant.
///
/// It matches `POST /login/oauth2/code/{registrationId}`, resolves the saved
/// `OAuth2AuthorizationRequest`, builds an `OAuth2AuthorizationCodeAuthenticationToken`
/// and authenticates it via the `AuthenticationManager`. On success it persists
/// the `OAuth2AuthorizedClient` and establishes the security context, then
/// redirects to the originally requested resource.
#[derive(Clone)]
pub struct OAuth2AuthorizationCodeGrantFilter {
    authorization_response_base_uri: String,
    authentication_manager: Option<Arc<dyn AuthenticationManager>>,
    client_registration_repository: Arc<dyn ClientRegistrationRepository>,
    authorized_client_repository: Arc<dyn OAuth2AuthorizedClientRepository>,
    authorization_request_repository: Arc<dyn AuthorizationRequestRepository>,
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    security_context_repository: Arc<dyn SecurityContextRepository>,
    request_cache: Arc<dyn RequestCache>,
    authorization_redirect_strategy: Arc<dyn RedirectStrategy>,
}

impl OAuth2AuthorizationCodeGrantFilter {
    /// Creates a new `OAuth2AuthorizationCodeGrantFilter` using the default
    /// authorization response base URI (`/login/oauth2/code`).
    pub fn new(
        client_registration_repository: Arc<dyn ClientRegistrationRepository>,
        authorized_client_repository: Arc<dyn OAuth2AuthorizedClientRepository>,
    ) -> Self {
        Self::with_base_uri(
            client_registration_repository,
            authorized_client_repository,
            DEFAULT_AUTHORIZATION_RESPONSE_BASE_URI,
        )
    }

    /// Creates a new `OAuth2AuthorizationCodeGrantFilter` using a custom
    /// authorization response base URI.
    pub fn with_base_uri(
        client_registration_repository: Arc<dyn ClientRegistrationRepository>,
        authorized_client_repository: Arc<dyn OAuth2AuthorizedClientRepository>,
        authorization_response_base_uri: &str,
    ) -> Self {
        assert!(
            !authorization_response_base_uri.trim().is_empty(),
            "authorization_response_base_uri cannot be empty"
        );
        Self {
            authorization_response_base_uri: authorization_response_base_uri.to_string(),
            authentication_manager: None,
            client_registration_repository,
            authorized_client_repository,
            authorization_request_repository: Arc::new(
                HttpSessionOAuth2AuthorizationRequestRepository::default(),
            ),
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            security_context_repository: Arc::new(
                RequestAttributeSecurityContextRepository::default(),
            ),
            request_cache: Arc::new(HttpSessionRequestCache::default()),
            authorization_redirect_strategy: Arc::new(DefaultRedirectStrategy::default()),
        }
    }

    pub fn set_authorization_request_repository(
        &mut self,
        authorization_request_repository: Arc<dyn AuthorizationRequestRepository>,
    ) {
        self.authorization_request_repository = authorization_request_repository;
    }

    pub fn set_authorization_redirect_strategy(
        &mut self,
        authorization_redirect_strategy: Arc<dyn RedirectStrategy>,
    ) {
        self.authorization_redirect_strategy = authorization_redirect_strategy;
    }

    pub fn set_request_cache(&mut self, request_cache: Arc<dyn RequestCache>) {
        self.request_cache = request_cache;
    }

    pub fn set_security_context_repository(
        &mut self,
        security_context_repository: Arc<dyn SecurityContextRepository>,
    ) {
        self.security_context_repository = security_context_repository;
    }

    pub fn set_security_context_holder_strategy(
        &mut self,
        security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = security_context_holder_strategy;
    }

    pub fn set_authentication_manager(
        &mut self,
        authentication_manager: Arc<dyn AuthenticationManager>,
    ) {
        self.authentication_manager = Some(authentication_manager);
    }

    /// Returns whether the request carries the authorization response of the
    /// authorization request that was saved when the authorization started.
    ///
    /// The response is recognized by the parameters it carries and by the
    /// redirect URI of the saved request, and not by the base URI of the filter
    /// alone: the filter serves the redirect URI the client declared, which may
    /// be any URI of the application.
    fn matches(&self, request: &dyn HttpRequest) -> bool {
        // An authorization response carries the code, or the error the
        // authorization endpoint reported.
        if request.parameter("code").is_none() && request.parameter("error").is_none() {
            return false;
        }

        let Some(authorization_request) = self
            .authorization_request_repository
            .load_authorization_request(request)
        else {
            return false;
        };

        match authorization_request.redirect_uri() {
            Some(redirect_uri) => redirect_uri_path(redirect_uri) == request.path(),
            // The saved request declares no redirect URI, so the base URI of
            // the filter is the URI the response is expected at.
            None => self.matches_base_uri(request),
        }
    }

    /// Returns whether the request targets the base URI of the filter.
    ///
    /// # Arguments
    ///
    /// * `request` - The request to match.
    fn matches_base_uri(&self, request: &dyn HttpRequest) -> bool {
        let prefix = format!(
            "{}/",
            self.authorization_response_base_uri.trim_end_matches('/')
        );
        request.path().starts_with(&prefix)
    }

    async fn successful(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: &Arc<dyn Authentication>,
    ) -> Result<(), FilterError> {
        if let Some(authorized_client) = authentication
            .principal()
            .and_then(|principal| principal.as_any().downcast_ref::<OAuth2AuthorizedClient>())
            .cloned()
        {
            self.authorized_client_repository.save_authorized_client(
                authorized_client,
                authentication.as_ref(),
                request,
                response,
            );
        }

        let context = self.security_context_holder_strategy.create_empty_context();
        context.set_authentication(Some(authentication.clone()));
        self.security_context_holder_strategy
            .set_context(context.clone());
        self.security_context_repository
            .save_context(&context, request, response)
            .await;

        let target = self
            .request_cache
            .get_request(request, response)
            .map(|saved| saved.get_redirect_url())
            .unwrap_or_else(|| "/".to_string());
        self.authorization_redirect_strategy
            .send_redirect(request, response, &target)
            .map_err(FilterError::from)?;
        Ok(())
    }

    fn unsuccessful(
        &self,
        _request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _error: AuthenticationError,
    ) -> Result<(), FilterError> {
        self.security_context_holder_strategy.clear_context();
        response.set_status_code(StatusCode::UNAUTHORIZED);
        response.commit();
        Ok(())
    }
}

#[async_trait]
impl HttpFilter for OAuth2AuthorizationCodeGrantFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if !self.matches(request) {
            return filter_chain.do_filter(request, response).await;
        }

        let authorization_request = match self
            .authorization_request_repository
            .remove_authorization_request(request)
        {
            Some(authorization_request) => authorization_request,
            None => {
                return self.unsuccessful(
                    request,
                    response,
                    AuthenticationError::with_kind(
                        "OAuth2 authorization request was not found",
                        AuthenticationErrorKind::BadCredentials,
                    ),
                );
            }
        };

        let authorization_response = if let Some(error) = request_parameter(request, "error") {
            let description = request_parameter(request, "error_description")
                .map(|description| format!(": {description}"))
                .unwrap_or_default();
            OAuth2AuthorizationResponse::error(
                error,
                Some(description),
                request_parameter(request, "state"),
            )
        } else {
            let code = match request_parameter(request, "code") {
                Some(code) => code,
                None => {
                    return self.unsuccessful(
                        request,
                        response,
                        AuthenticationError::with_kind(
                            "OAuth2 authorization response did not contain a code",
                            AuthenticationErrorKind::BadCredentials,
                        ),
                    );
                }
            };
            OAuth2AuthorizationResponse::success(code, request_parameter(request, "state"))
        };

        if authorization_response.is_error() {
            return self.unsuccessful(
                request,
                response,
                AuthenticationError::with_kind(
                    format!(
                        "OAuth2 authorization response contained error: {}",
                        authorization_response.error_code().unwrap_or_default()
                    ),
                    AuthenticationErrorKind::BadCredentials,
                ),
            );
        }

        let state = authorization_response.state().map(str::to_string);
        if authorization_request.state() != state.as_deref().unwrap_or("") {
            return self.unsuccessful(
                request,
                response,
                AuthenticationError::with_kind(
                    "OAuth2 authorization response state did not match the authorization request",
                    AuthenticationErrorKind::BadCredentials,
                ),
            );
        }

        let client_registration = match self
            .client_registration_repository
            .find_by_registration_id(authorization_request.registration_id())
        {
            Some(client_registration) => client_registration,
            None => {
                return self.unsuccessful(
                    request,
                    response,
                    AuthenticationError::with_kind(
                        "OAuth2 client registration was not found",
                        AuthenticationErrorKind::BadCredentials,
                    ),
                );
            }
        };

        let authorization_exchange =
            OAuth2AuthorizationExchange::new(authorization_request, authorization_response);
        let authentication = OAuth2AuthorizationCodeAuthenticationToken::unauthenticated(
            client_registration,
            authorization_exchange,
        );

        let authentication_manager = match self.authentication_manager.clone() {
            Some(authentication_manager) => authentication_manager,
            None => {
                return self.unsuccessful(
                    request,
                    response,
                    AuthenticationError::with_kind(
                        "AuthenticationManager is required for oauth2 client",
                        AuthenticationErrorKind::AuthenticationService,
                    ),
                );
            }
        };

        match authentication_manager.authenticate(&authentication).await {
            Ok(authenticated) => self.successful(request, response, &authenticated).await,
            Err(error) => self.unsuccessful(request, response, error),
        }
    }
}

impl Named for OAuth2AuthorizationCodeGrantFilter {
    fn name(&self) -> &str {
        "OAuth2AuthorizationCodeGrantFilter"
    }
}

impl fmt::Debug for OAuth2AuthorizationCodeGrantFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OAuth2AuthorizationCodeGrantFilter")
            .field(
                "authorization_response_base_uri",
                &self.authorization_response_base_uri,
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        },
    };

    use axum::{body::Body, extract::Request, response::Response};
    use next_web_core::{
        async_trait,
        filter::FilterError,
        http::StatusCode,
        traits::{
            filter::{HttpFilter, HttpFilterChain},
            http::{http_request::HttpRequest, http_response::HttpResponse},
        },
    };

    use super::OAuth2AuthorizationCodeGrantFilter;
    use crate::{
        authentication::UsernamePasswordAuthenticationToken,
        authorization::AuthenticationManager,
        core::{Authentication, AuthenticationError},
        oauth2::{
            AuthorizationRequestRepository, ClientRegistration,
            InMemoryClientRegistrationRepository, OAuth2AccessToken, OAuth2AuthorizationRequest,
            OAuth2AuthorizedClient, OAuth2AuthorizedClientRepository,
        },
        web::savedrequest::NullRequestCache,
    };

    /// The path the `github` registration declares as its redirect URI.
    const GITHUB_REDIRECT_URI: &str = "/login/oauth2/code/github";

    /// A repository that records the authorization request it is given, and
    /// that removes it the way the filter does.
    #[derive(Clone, Default)]
    struct RecordingAuthorizationRequestRepository {
        saved: Arc<Mutex<Option<OAuth2AuthorizationRequest>>>,
    }

    impl AuthorizationRequestRepository for RecordingAuthorizationRequestRepository {
        fn load_authorization_request(
            &self,
            _request: &dyn HttpRequest,
        ) -> Option<OAuth2AuthorizationRequest> {
            self.saved.lock().unwrap().clone()
        }

        fn save_authorization_request(
            &self,
            authorization_request: Option<OAuth2AuthorizationRequest>,
            _request: &mut dyn HttpRequest,
            _response: &mut dyn HttpResponse,
        ) {
            *self.saved.lock().unwrap() = authorization_request;
        }

        fn remove_authorization_request(
            &self,
            _request: &mut dyn HttpRequest,
        ) -> Option<OAuth2AuthorizationRequest> {
            self.saved.lock().unwrap().take()
        }
    }

    /// A repository that records the authorized client it is given.
    #[derive(Clone, Default)]
    struct RecordingAuthorizedClientRepository {
        saved: Arc<Mutex<Option<OAuth2AuthorizedClient>>>,
    }

    impl OAuth2AuthorizedClientRepository for RecordingAuthorizedClientRepository {
        fn save_authorized_client(
            &self,
            authorized_client: OAuth2AuthorizedClient,
            _principal: &dyn Authentication,
            _request: &dyn HttpRequest,
            _response: &mut dyn HttpResponse,
        ) {
            *self.saved.lock().unwrap() = Some(authorized_client);
        }
    }

    /// An authentication manager that authenticates every client.
    #[derive(Clone)]
    struct TestAuthenticationManager {
        authorized_client: OAuth2AuthorizedClient,
    }

    #[async_trait]
    impl AuthenticationManager for TestAuthenticationManager {
        async fn authenticate(
            &self,
            _authentication: &dyn Authentication,
        ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
            Ok(Arc::new(
                UsernamePasswordAuthenticationToken::authenticated(
                    Arc::new(self.authorized_client.clone()),
                    None,
                    Vec::new(),
                ),
            ))
        }
    }

    /// A chain that records that it ran.
    #[derive(Clone, Default)]
    struct RecordingChain {
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl HttpFilterChain for RecordingChain {
        async fn do_filter(
            &self,
            _request: &mut dyn HttpRequest,
            _response: &mut dyn HttpResponse,
        ) -> Result<(), FilterError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    /// A registration whose redirect URI is the callback of the filter.
    fn registration() -> ClientRegistration {
        ClientRegistration::new("github", "GitHub", "https://example.com/oauth2/authorize")
            .set_client_id("client-id")
            .set_redirect_uri(GITHUB_REDIRECT_URI)
    }

    /// A saved authorization request whose state is `state`.
    fn authorization_request(state: &str) -> OAuth2AuthorizationRequest {
        OAuth2AuthorizationRequest::new(
            "github",
            "https://example.com/oauth2/authorize",
            state,
            Some(GITHUB_REDIRECT_URI.to_string()),
            Vec::new(),
        )
    }

    /// A filter whose authentication manager authenticates every client.
    fn filter(
        authorization_request: Option<OAuth2AuthorizationRequest>,
        authorized_clients: &RecordingAuthorizedClientRepository,
    ) -> (
        OAuth2AuthorizationCodeGrantFilter,
        RecordingAuthorizationRequestRepository,
    ) {
        let repository = RecordingAuthorizationRequestRepository {
            saved: Arc::new(Mutex::new(authorization_request)),
        };
        let mut filter = OAuth2AuthorizationCodeGrantFilter::new(
            Arc::new(InMemoryClientRegistrationRepository::new([registration()])),
            Arc::new(authorized_clients.clone()),
        );
        filter.set_authorization_request_repository(Arc::new(repository.clone()));
        filter.set_authentication_manager(Arc::new(TestAuthenticationManager {
            authorized_client: OAuth2AuthorizedClient::new(
                registration(),
                "user",
                OAuth2AccessToken::new("token"),
            ),
        }));
        // The request cache of a test does not have a session to save a request
        // in, and the filter falls back to the root of the application.
        filter.set_request_cache(Arc::new(NullRequestCache::default()));

        (filter, repository)
    }

    fn request(uri: &str) -> Request {
        Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn continues_the_chain_when_the_request_is_not_an_authorization_response() {
        let (filter, _) = filter(Some(authorization_request("state-1")), &Default::default());
        let mut request = request(GITHUB_REDIRECT_URI);
        let mut response = Response::new(Body::empty());
        let chain = RecordingChain::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .unwrap();

        assert_eq!(chain.calls.load(Ordering::SeqCst), 1);
        assert_eq!(response.status_code(), StatusCode::OK);
    }

    #[tokio::test]
    async fn continues_the_chain_when_no_authorization_request_was_saved() {
        let (filter, _) = filter(None, &Default::default());
        let mut request = request(&format!("{GITHUB_REDIRECT_URI}?code=code-1&state=state-1"));
        let mut response = Response::new(Body::empty());
        let chain = RecordingChain::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .unwrap();

        assert_eq!(chain.calls.load(Ordering::SeqCst), 1);
        assert_eq!(response.status_code(), StatusCode::OK);
    }

    #[tokio::test]
    async fn continues_the_chain_when_the_request_targets_another_redirect_uri() {
        let mut saved = authorization_request("state-1");
        saved = OAuth2AuthorizationRequest::new(
            saved.registration_id(),
            saved.authorization_request_uri(),
            saved.state(),
            Some("/other/callback".to_string()),
            Vec::new(),
        );
        let (filter, _) = filter(Some(saved), &Default::default());
        let mut request = request(&format!("{GITHUB_REDIRECT_URI}?code=code-1&state=state-1"));
        let mut response = Response::new(Body::empty());
        let chain = RecordingChain::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .unwrap();

        assert_eq!(chain.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn rejects_a_response_whose_state_does_not_match() {
        let (filter, _) = filter(Some(authorization_request("state-1")), &Default::default());
        let mut request = request(&format!("{GITHUB_REDIRECT_URI}?code=code-1&state=other"));
        let mut response = Response::new(Body::empty());
        let chain = RecordingChain::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .unwrap();

        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(chain.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn completes_the_login_and_saves_the_authorized_client() {
        let authorized_clients = RecordingAuthorizedClientRepository::default();
        let (filter, repository) =
            filter(Some(authorization_request("state-1")), &authorized_clients);
        let mut request = request(&format!("{GITHUB_REDIRECT_URI}?code=code-1&state=state-1"));
        let mut response = Response::new(Body::empty());
        let chain = RecordingChain::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .unwrap();

        assert_eq!(response.status_code(), StatusCode::FOUND);
        assert!(
            authorized_clients.saved.lock().unwrap().is_some(),
            "the authorized client is saved"
        );
        assert!(
            repository.saved.lock().unwrap().is_none(),
            "the authorization request is removed"
        );
        assert_eq!(chain.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn reads_the_path_of_a_redirect_uri() {
        assert_eq!(
            super::redirect_uri_path("https://example.com/login/oauth2/code/github?x=1"),
            "/login/oauth2/code/github"
        );
        assert_eq!(
            super::redirect_uri_path("/login/oauth2/code/github"),
            "/login/oauth2/code/github"
        );
        assert_eq!(super::redirect_uri_path("https://example.com"), "/");
    }
}
