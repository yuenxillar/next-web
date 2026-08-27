use std::{
    error::Error,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use next_web_core::{
    anys::any_value::AnyValue,
    async_trait,
    error::BoxError,
    filter::FilterError,
    http::{HttpMethod, StatusCode},
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};
use tracing::{error, warn};

use crate::{
    core::{AuthenticationError, AuthenticationErrorKind},
    oauth2::{
        AuthorizationGrantType, AuthorizationRequestRepository, ClientAuthorizationRequiredError,
        ClientRegistrationRepository, DefaultOAuth2AuthorizationRequestResolver,
        HttpSessionOAuth2AuthorizationRequestRepository, OAuth2AuthorizationRequest,
        OAuth2AuthorizationRequestResolver,
    },
    web::{
        authentication::AuthenticationFailureHandler,
        savedrequest::{HttpSessionRequestCache, RequestCache},
        DefaultRedirectStrategy, RedirectStrategy,
    },
};

/// Global counter used to generate unique per-instance filter keys.
/// This mirrors `OncePerRequestFilter`'s behavior where each instance
/// gets a unique attribute name based on `System.identityHashCode(this)`.
static FILTER_INSTANCE_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A filter that initiates an OAuth 2.0 Authorization Request if the current
/// request matches the authorization request base URI.
#[derive(Clone)]
pub struct OAuth2AuthorizationRequestRedirectFilter {
    authorization_request_resolver: Arc<dyn OAuth2AuthorizationRequestResolver>,
    authorization_request_repository: Arc<dyn AuthorizationRequestRepository>,
    authorization_redirect_strategy: Arc<dyn RedirectStrategy>,
    request_cache: Arc<dyn RequestCache>,
    authentication_failure_handler: Arc<dyn AuthenticationFailureHandler>,
    /// Unique per-instance attribute key to ensure this filter runs only once
    /// per request. Mirrors `OncePerRequestFilter.getAlreadyFilteredAttributeName()`.
    already_filtered_attribute_name: String,
}

impl OAuth2AuthorizationRequestRedirectFilter {
    /// The default base `URI` used for authorization requests.
    pub const DEFAULT_AUTHORIZATION_REQUEST_BASE_URI: &'static str = "/oauth2/authorization";

    /// The suffix appended to the already-filtered attribute name, matching
    /// the `OncePerRequestFilter.ALREADY_FILTERED_SUFFIX`.
    const ALREADY_FILTERED_SUFFIX: &'static str = ".FILTERED";

    /// Creates a filter using the provided `OAuth2AuthorizationRequestResolver`.
    pub fn new(
        authorization_request_resolver: Arc<dyn OAuth2AuthorizationRequestResolver>,
    ) -> Self {
        let instance_id = FILTER_INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self {
            authorization_request_resolver,
            authorization_request_repository: Arc::new(
                HttpSessionOAuth2AuthorizationRequestRepository::default(),
            ),
            authorization_redirect_strategy: Arc::new(DefaultRedirectStrategy::default()),
            request_cache: Arc::new(HttpSessionRequestCache::default()),
            authentication_failure_handler: Arc::new(DefaultAuthorizationFailureHandler),
            already_filtered_attribute_name: format!(
                "{}-{}{}",
                std::any::type_name::<Self>(),
                instance_id,
                Self::ALREADY_FILTERED_SUFFIX
            ),
        }
    }

    /// Creates a filter using a `DefaultOAuth2AuthorizationRequestResolver`
    /// backed by the given `ClientRegistrationRepository` and the default
    /// authorization request base URI.
    pub fn with_client_registration_repository(
        client_registration_repository: Arc<dyn ClientRegistrationRepository>,
    ) -> Self {
        Self::with_client_registration_repository_and_base_uri(
            client_registration_repository,
            Self::DEFAULT_AUTHORIZATION_REQUEST_BASE_URI,
        )
    }

    /// Creates a filter using a `DefaultOAuth2AuthorizationRequestResolver`
    /// backed by the given `ClientRegistrationRepository` and a custom
    /// authorization request base URI.
    pub fn with_client_registration_repository_and_base_uri(
        client_registration_repository: Arc<dyn ClientRegistrationRepository>,
        authorization_request_base_uri: impl Into<String>,
    ) -> Self {
        Self::new(Arc::new(DefaultOAuth2AuthorizationRequestResolver::new(
            client_registration_repository,
            authorization_request_base_uri,
        )))
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

    pub fn set_authentication_failure_handler(
        &mut self,
        authentication_failure_handler: Arc<dyn AuthenticationFailureHandler>,
    ) {
        self.authentication_failure_handler = authentication_failure_handler;
    }

    /// Saves the authorization request (for the authorization code grant only)
    /// and redirects to the pre-built authorization request URI.
    fn send_redirect_for_authorization(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authorization_request: OAuth2AuthorizationRequest,
    ) -> Result<(), FilterError> {
        let redirect_uri = authorization_request.authorization_request_uri().to_string();
        if authorization_request.grant_type() == &AuthorizationGrantType::AuthorizationCode {
            self.authorization_request_repository
                .save_authorization_request(Some(authorization_request), request, response);
        }
        self.authorization_redirect_strategy
            .send_redirect(request, response, &redirect_uri)
            .map_err(FilterError::from)?;
        Ok(())
    }

    /// Delegates an authorization request failure to the configured
    /// `AuthenticationFailureHandler`.
    fn on_authentication_failure(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        error: AuthenticationError,
    ) -> Result<(), FilterError> {
        self.authentication_failure_handler
            .on_authentication_failure(request, response, &error)
            .map_err(FilterError::from)
    }
}

/// Default `AuthenticationFailureHandler` used when an authorization request
/// cannot be resolved. Logs at WARN for an invalid registration id (so those
/// errors can be tuned separately from other errors) and at ERROR otherwise,
/// then responds with HTTP 500.
#[derive(Clone, Debug, Default)]
struct DefaultAuthorizationFailureHandler;

impl AuthenticationFailureHandler for DefaultAuthorizationFailureHandler {
    fn on_authentication_failure(
        &self,
        _request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        error: &AuthenticationError,
    ) -> Result<(), BoxError> {
        if error.kind() == AuthenticationErrorKind::InvalidClientRegistrationId {
            // Log an invalid registrationId at WARN level to allow these
            // errors to be tuned separately from other errors
            warn!("Authorization Request failed: {}", error.message());
        } else {
            error!("Authorization Request failed: {}", error.message());
        }
        response.set_status_code(StatusCode::INTERNAL_SERVER_ERROR);
        response.finish();
        Ok(())
    }
}

/// Walks the cause chain of the given `FilterError` looking for the first
/// `ClientAuthorizationRequiredError`. Only errors propagated through
/// `FilterChainError::AnyError` preserve their concrete type; errors wrapped
/// in a `FilterChainError::Boxed` are not detectable.
fn find_client_authorization_required_error(
    error: &FilterError,
) -> Option<&ClientAuthorizationRequiredError> {
    let mut current: &(dyn Error + 'static) = match error {
        FilterError::Chain(chain_error) => chain_error,
        _ => return None,
    };
    loop {
        if let Some(authorization_required) =
            current.downcast_ref::<ClientAuthorizationRequiredError>()
        {
            return Some(authorization_required);
        }
        let Some(source) = current.source() else {
            return None;
        };
        current = source;
    }
}

#[async_trait]
impl HttpFilter for OAuth2AuthorizationRequestRedirectFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if request.method() != HttpMethod::GET {
            return filter_chain.do_filter(request, response).await;
        }

        // Ensure the filter is only applied once per request (OncePerRequestFilter).
        if request
            .get_attribute(&self.already_filtered_attribute_name)
            .is_some()
        {
            return filter_chain.do_filter(request, response).await;
        }
        request.set_attribute(&self.already_filtered_attribute_name, AnyValue::Boolean(true));

        // Attempt to resolve the OAuth 2.0 Authorization Request.
        match self.authorization_request_resolver.resolve(request) {
            Ok(Some(authorization_request)) => {
                return self.send_redirect_for_authorization(request, response, authorization_request);
            }
            Ok(None) => {}
            Err(error) => {
                return self.on_authentication_failure(request, response, error);
            }
        }

        // Not an authorization request: continue the chain and watch for a
        // ClientAuthorizationRequiredError deep in the cause chain.
        match filter_chain.do_filter(request, response).await {
            Ok(()) => Ok(()),
            Err(error) => {
                let Some(authorization_required) = find_client_authorization_required_error(&error)
                else {
                    return Err(error);
                };
                let registration_id = authorization_required.client_registration_id().to_string();
                match self
                    .authorization_request_resolver
                    .resolve_with_registration_id(request, &registration_id)
                {
                    Ok(Some(authorization_request)) => {
                        self.request_cache.save_request(request, response);
                        self.send_redirect_for_authorization(
                            request,
                            response,
                            authorization_request,
                        )
                    }
                    Ok(None) => {
                        // Mirrors rethrowing the ClientAuthorizationRequiredException,
                        // which is then wrapped and forwarded to the failure handler.
                        let failure = AuthenticationError::with_kind(
                            format!(
                                "Authorization required for Client Registration Id: {}",
                                registration_id
                            ),
                            AuthenticationErrorKind::General,
                        );
                        self.on_authentication_failure(request, response, failure)
                    }
                    Err(error) => self.on_authentication_failure(request, response, error),
                }
            }
        }
    }
}

impl Named for OAuth2AuthorizationRequestRedirectFilter {
    fn name(&self) -> &str {
        "OAuth2AuthorizationRequestRedirectFilter"
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{atomic::AtomicBool, Mutex};

    use axum::{
        body::Body,
        extract::Request as AxumRequest,
        http::Version,
        response::Response as AxumResponse,
    };
    use next_web_core::{
        filter::FilterChainError,
        http::{Cookie, HttpResponseShare},
        traits::http::{http_request::HttpRequest, http_response::HttpResponse},
    };

    use super::*;
    use crate::{
        oauth2::{ClientRegistration, InMemoryClientRegistrationRepository},
        web::savedrequest::SavedRequest,
    };

    /// Wraps `axum::response::Response` with a no-op `finish()` because the
    /// axum implementation of `finish()`/`is_committed()` panics with `todo!()`.
    struct TestResponse {
        inner: AxumResponse,
    }

    impl Default for TestResponse {
        fn default() -> Self {
            Self {
                inner: AxumResponse::new(Body::empty()),
            }
        }
    }

    impl HttpResponse for TestResponse {
        fn version(&self) -> Version {
            HttpResponse::version(&self.base)
        }

        fn status_code(&self) -> StatusCode {
            HttpResponse::status_code(&self.base)
        }

        fn set_status_code(&mut self, status_code: StatusCode) {
            HttpResponse::set_status_code(&mut self.base, status_code);
        }

        fn header(&self, name: &str) -> Option<&str> {
            HttpResponse::header(&self.base, name)
        }

        fn headers(&self, name: &str) -> Option<Vec<&str>> {
            HttpResponse::headers(&self.base, name)
        }

        fn append_header(&mut self, name: &str, value: &str) -> bool {
            HttpResponse::append_header(&mut self.base, name, value)
        }

        fn contains_header(&self, name: &str) -> bool {
            HttpResponse::contains_header(&self.base, name)
        }

        fn insert_header(&mut self, name: &str, value: &str) -> Option<String> {
            HttpResponse::insert_header(&mut self.base, name, value)
        }

        fn remove_header(&mut self, name: &str) -> Option<String> {
            HttpResponse::remove_header(&mut self.base, name)
        }

        fn set_body(&mut self, body: Vec<u8>) {
            HttpResponse::set_body(&mut self.base, body);
        }

        fn set_redirect(&mut self, url: &str) {
            HttpResponse::set_redirect(&mut self.base, url);
        }

        fn add_cookie(&mut self, cookie: Cookie) {
            HttpResponse::add_cookie(&mut self.base, cookie);
        }

        fn is_committed(&self) -> bool {
            false
        }

        fn finish(&mut self) {}

        fn shared(&mut self) -> &HttpResponseShare {
            HttpResponse::shared(&mut self.base)
        }
    }

    /// Records `save_authorization_request` invocations without touching the
    /// session (axum test requests have no session support).
    #[derive(Clone, Default)]
    struct RecordingAuthorizationRequestRepository {
        saved: Arc<Mutex<Option<OAuth2AuthorizationRequest>>>,
    }

    impl AuthorizationRequestRepository for RecordingAuthorizationRequestRepository {
        fn load_authorization_request(
            &self,
            _request: &dyn HttpRequest,
        ) -> Option<OAuth2AuthorizationRequest> {
            None
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
            _request: &dyn HttpRequest,
        ) -> Option<OAuth2AuthorizationRequest> {
            None
        }
    }

    #[derive(Clone)]
    enum ChainOutcome {
        Continue,
        ThrowClientAuthorizationRequired(String),
        ThrowNestedClientAuthorizationRequired(String),
    }

    /// A filter chain that records its invocation and optionally throws a
    /// `ClientAuthorizationRequiredError` (directly or nested in the cause chain).
    #[derive(Clone)]
    struct TestFilterChain {
        invoked: Arc<AtomicBool>,
        outcome: ChainOutcome,
    }

    impl TestFilterChain {
        fn new(outcome: ChainOutcome) -> Self {
            Self {
                invoked: Arc::new(AtomicBool::new(false)),
                outcome,
            }
        }

        fn was_invoked(&self) -> bool {
            self.invoked.load(Ordering::SeqCst)
        }
    }

    impl Named for TestFilterChain {
        fn name(&self) -> &str {
            "TestFilterChain"
        }
    }

    #[async_trait]
    impl HttpFilterChain for TestFilterChain {
        async fn do_filter(
            &self,
            _request: &mut dyn HttpRequest,
            _response: &mut dyn HttpResponse,
        ) -> Result<(), FilterError> {
            self.invoked.store(true, Ordering::SeqCst);
            match &self.outcome {
                ChainOutcome::Continue => Ok(()),
                ChainOutcome::ThrowClientAuthorizationRequired(registration_id) => {
                    Err(ClientAuthorizationRequiredError::new(registration_id.clone()).into())
                }
                ChainOutcome::ThrowNestedClientAuthorizationRequired(registration_id) => {
                    Err(NestedError {
                        inner: ClientAuthorizationRequiredError::new(registration_id.clone()),
                    }
                    .into())
                }
            }
        }
    }

    /// Wraps a `ClientAuthorizationRequiredError` to exercise the cause-chain walk.
    #[derive(Debug)]
    struct NestedError {
        inner: ClientAuthorizationRequiredError,
    }

    impl std::fmt::Display for NestedError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "nested: {}", self.base)
        }
    }

    impl Error for NestedError {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Some(&self.base)
        }
    }

    impl From<NestedError> for FilterError {
        fn from(value: NestedError) -> Self {
            FilterError::Chain(FilterChainError::AnyError(Box::new(value)))
        }
    }

    /// Records `save_request` invocations.
    #[derive(Clone, Default)]
    struct RecordingRequestCache {
        saved: Arc<AtomicBool>,
    }

    impl RequestCache for RecordingRequestCache {
        fn save_request(&self, _request: &mut dyn HttpRequest, _response: &mut dyn HttpResponse) {
            self.saved.store(true, Ordering::SeqCst);
        }

        fn get_request(
            &self,
            _request: &dyn HttpRequest,
            _response: &mut dyn HttpResponse,
        ) -> Option<Arc<dyn SavedRequest>> {
            None
        }

        fn get_matching_request<'a>(
            &self,
            _request: &'a mut dyn HttpRequest,
            _response: &mut dyn HttpResponse,
        ) -> Option<Box<dyn HttpRequest + 'a>> {
            None
        }

        fn remove_request(&self, _request: &dyn HttpRequest, _response: &mut dyn HttpResponse) {}
    }

    /// A redirect strategy that captures the redirect URL and responds with 200.
    #[derive(Clone, Default)]
    struct CapturingRedirectStrategy {
        captured_url: Arc<Mutex<Option<String>>>,
    }

    impl RedirectStrategy for CapturingRedirectStrategy {
        fn send_redirect(
            &self,
            _request: &dyn HttpRequest,
            response: &mut dyn HttpResponse,
            url: &str,
        ) -> Result<(), BoxError> {
            *self.captured_url.lock().unwrap() = Some(url.to_string());
            response.set_status_code(StatusCode::OK);
            Ok(())
        }
    }

    /// A failure handler that records the error kind and responds with a
    /// configurable status code.
    #[derive(Clone, Debug)]
    struct StatusSettingFailureHandler {
        status: StatusCode,
        seen_kind: Arc<Mutex<Option<AuthenticationErrorKind>>>,
    }

    impl StatusSettingFailureHandler {
        fn new(status: StatusCode) -> Self {
            Self {
                status,
                seen_kind: Arc::new(Mutex::new(None)),
            }
        }
    }

    impl AuthenticationFailureHandler for StatusSettingFailureHandler {
        fn on_authentication_failure(
            &self,
            _request: &mut dyn HttpRequest,
            response: &mut dyn HttpResponse,
            error: &AuthenticationError,
        ) -> Result<(), BoxError> {
            *self.seen_kind.lock().unwrap() = Some(error.kind());
            response.set_status_code(self.status);
            Ok(())
        }
    }

    /// A resolver with fixed results for both resolve methods.
    #[derive(Clone)]
    struct FixedRequestResolver {
        resolve_result: Result<Option<OAuth2AuthorizationRequest>, AuthenticationError>,
        resolve_with_registration_id_result: Result<Option<OAuth2AuthorizationRequest>, AuthenticationError>,
        seen_registration_id: Arc<Mutex<Option<String>>>,
    }

    impl Default for FixedRequestResolver {
        fn default() -> Self {
            Self {
                resolve_result: Ok(None),
                resolve_with_registration_id_result: Ok(None),
                seen_registration_id: Arc::new(Mutex::new(None)),
            }
        }
    }

    impl OAuth2AuthorizationRequestResolver for FixedRequestResolver {
        fn resolve(
            &self,
            _request: &dyn HttpRequest,
        ) -> Result<Option<OAuth2AuthorizationRequest>, AuthenticationError> {
            self.resolve_result.clone()
        }

        fn resolve_with_registration_id(
            &self,
            _request: &dyn HttpRequest,
            client_registration_id: &str,
        ) -> Result<Option<OAuth2AuthorizationRequest>, AuthenticationError> {
            *self.seen_registration_id.lock().unwrap() = Some(client_registration_id.to_string());
            self.resolve_with_registration_id_result.clone()
        }
    }

    fn get_request(path: &str) -> AxumRequest {
        let mut request = AxumRequest::builder()
            .uri(path)
            .body(Body::empty())
            .unwrap();
        request.ready();
        request
    }

    fn post_request(path: &str) -> AxumRequest {
        let mut request = AxumRequest::builder()
            .method("POST")
            .uri(path)
            .body(Body::empty())
            .unwrap();
        request.ready();
        request
    }

    fn client_registration(registration_id: &str) -> ClientRegistration {
        ClientRegistration::new(
            registration_id,
            "Test Client",
            "https://example.com/login/oauth/authorize",
        )
        .set_client_id("client-id")
        .set_scopes(["read:user"])
        .set_redirect_uri(format!("http://localhost/login/oauth2/code/{}", registration_id))
    }

    /// Builds the expected authorization request URI prefix. The random `state`
    /// parameter is appended by the resolver. Note that the Rust URI builder
    /// percent-encodes every value and emits no PKCE parameters.
    fn expected_redirect_uri(registration_id: &str) -> String {
        format!(
            "https://example.com/login/oauth/authorize?response_type=code&client_id={}&scope={}&redirect_uri={}&state=",
            urlencoding::encode("client-id"),
            urlencoding::encode("read:user"),
            urlencoding::encode(&format!(
                "http://localhost/login/oauth2/code/{}",
                registration_id
            )),
        )
    }

    /// Builds a filter backed by a single valid `registration-id` registration,
    /// with a recording repository (axum test requests panic on session access).
    fn redirect_filter() -> OAuth2AuthorizationRequestRedirectFilter {
        let repository: Arc<dyn ClientRegistrationRepository> = Arc::new(
            InMemoryClientRegistrationRepository::new([client_registration("registration-id")]),
        );
        let mut filter =
            OAuth2AuthorizationRequestRedirectFilter::with_client_registration_repository(repository);
        filter.set_authorization_request_repository(Arc::new(
            RecordingAuthorizationRequestRepository::default(),
        ));
        filter
    }

    #[tokio::test]
    async fn chain_continues_when_not_authorization_request() {
        let filter = redirect_filter();
        let chain = TestFilterChain::new(ChainOutcome::Continue);
        let mut request = get_request("/path");
        let mut response = TestResponse::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .unwrap();

        assert!(chain.was_invoked());
        assert!(response.header("location").is_none());
    }

    #[tokio::test]
    async fn authorization_request_with_invalid_client_returns_500() {
        let filter = redirect_filter();
        let chain = TestFilterChain::new(ChainOutcome::Continue);
        let mut request = get_request("/oauth2/authorization/registration-id-invalid");
        let mut response = TestResponse::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .unwrap();

        assert!(!chain.was_invoked());
        assert_eq!(response.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn authorization_request_with_invalid_client_and_custom_failure_handler_returns_custom_status()
    {
        let mut filter = redirect_filter();
        let failure_handler = StatusSettingFailureHandler::new(StatusCode::BAD_REQUEST);
        filter.set_authentication_failure_handler(Arc::new(failure_handler.clone()));
        let chain = TestFilterChain::new(ChainOutcome::Continue);
        let mut request = get_request("/oauth2/authorization/registration-id-invalid");
        let mut response = TestResponse::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .unwrap();

        assert!(!chain.was_invoked());
        assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(
            *failure_handler.seen_kind.lock().unwrap(),
            Some(AuthenticationErrorKind::InvalidClientRegistrationId)
        );
    }

    #[tokio::test]
    async fn authorization_request_redirects_for_authorization() {
        let filter = redirect_filter();
        let chain = TestFilterChain::new(ChainOutcome::Continue);
        let mut request = get_request("/oauth2/authorization/registration-id");
        let mut response = TestResponse::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .unwrap();

        assert!(!chain.was_invoked());
        assert_eq!(response.status_code(), StatusCode::SEE_OTHER);
        let location = response.header("location").unwrap();
        let prefix = expected_redirect_uri("registration-id");
        assert!(
            location.starts_with(&prefix),
            "expected location to start with {:?}, got {:?}",
            prefix,
            location
        );
        // The state parameter is a random UUID v4.
        assert_eq!(location.len() - prefix.len(), 36);
    }

    #[tokio::test]
    async fn authorization_request_is_saved_to_repository() {
        let repository: Arc<dyn ClientRegistrationRepository> = Arc::new(
            InMemoryClientRegistrationRepository::new([client_registration("registration-id")]),
        );
        let mut filter =
            OAuth2AuthorizationRequestRedirectFilter::with_client_registration_repository(repository);
        let recording_repository = RecordingAuthorizationRequestRepository::default();
        filter.set_authorization_request_repository(Arc::new(recording_repository.clone()));
        let chain = TestFilterChain::new(ChainOutcome::Continue);
        let mut request = get_request("/oauth2/authorization/registration-id");
        let mut response = TestResponse::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .unwrap();

        let saved = recording_repository.saved.lock().unwrap().clone();
        let saved = saved.expect("authorization request should be saved");
        assert_eq!(saved.registration_id(), "registration-id");
    }

    #[tokio::test]
    async fn custom_authorization_request_base_uri_redirects() {
        let repository: Arc<dyn ClientRegistrationRepository> = Arc::new(
            InMemoryClientRegistrationRepository::new([client_registration("registration-id")]),
        );
        let mut filter = OAuth2AuthorizationRequestRedirectFilter::with_client_registration_repository_and_base_uri(
            repository,
            "/custom/authorization",
        );
        filter.set_authorization_request_repository(Arc::new(
            RecordingAuthorizationRequestRepository::default(),
        ));
        let chain = TestFilterChain::new(ChainOutcome::Continue);
        let mut request = get_request("/custom/authorization/registration-id");
        let mut response = TestResponse::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .unwrap();

        assert!(!chain.was_invoked());
        let location = response.header("location").unwrap();
        assert!(location.starts_with(&expected_redirect_uri("registration-id")));
    }

    #[tokio::test]
    async fn client_authorization_required_from_chain_redirects_and_saves_request() {
        let mut filter = redirect_filter();
        let cache = RecordingRequestCache::default();
        filter.set_request_cache(Arc::new(cache.clone()));
        let chain = TestFilterChain::new(ChainOutcome::ThrowClientAuthorizationRequired(
            "registration-id".to_string(),
        ));
        let mut request = get_request("/path");
        let mut response = TestResponse::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .unwrap();

        assert!(chain.was_invoked());
        assert!(cache.saved.load(Ordering::SeqCst));
        assert_eq!(response.status_code(), StatusCode::SEE_OTHER);
        let location = response.header("location").unwrap();
        assert!(location.starts_with(&expected_redirect_uri("registration-id")));
    }

    #[tokio::test]
    async fn client_authorization_required_but_unresolved_returns_500() {
        let resolver = FixedRequestResolver::default();
        let cache = RecordingRequestCache::default();
        let mut filter = OAuth2AuthorizationRequestRedirectFilter::new(Arc::new(resolver.clone()));
        filter.set_request_cache(Arc::new(cache.clone()));
        let chain = TestFilterChain::new(ChainOutcome::ThrowClientAuthorizationRequired(
            "registration-id".to_string(),
        ));
        let mut request = get_request("/path");
        let mut response = TestResponse::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .unwrap();

        assert!(chain.was_invoked());
        assert!(!cache.saved.load(Ordering::SeqCst));
        assert_eq!(response.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            *resolver.seen_registration_id.lock().unwrap(),
            Some("registration-id".to_string())
        );
    }

    #[tokio::test]
    async fn nested_client_authorization_required_from_chain_redirects() {
        let mut filter = redirect_filter();
        let cache = RecordingRequestCache::default();
        filter.set_request_cache(Arc::new(cache.clone()));
        let chain = TestFilterChain::new(ChainOutcome::ThrowNestedClientAuthorizationRequired(
            "registration-id".to_string(),
        ));
        let mut request = get_request("/path");
        let mut response = TestResponse::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .unwrap();

        assert!(chain.was_invoked());
        assert!(cache.saved.load(Ordering::SeqCst));
        assert_eq!(response.status_code(), StatusCode::SEE_OTHER);
    }

    #[tokio::test]
    async fn custom_authorization_redirect_strategy_is_used() {
        let mut filter = redirect_filter();
        let redirect_strategy = CapturingRedirectStrategy::default();
        filter.set_authorization_redirect_strategy(Arc::new(redirect_strategy.clone()));
        let chain = TestFilterChain::new(ChainOutcome::Continue);
        let mut request = get_request("/oauth2/authorization/registration-id");
        let mut response = TestResponse::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .unwrap();

        assert_eq!(response.status_code(), StatusCode::OK);
        let captured = redirect_strategy
            .captured_url
            .lock()
            .unwrap()
            .clone()
            .unwrap();
        assert!(captured.starts_with(&expected_redirect_uri("registration-id")));
    }

    #[tokio::test]
    async fn non_authorization_code_grant_redirects_without_saving() {
        let authorization_request = OAuth2AuthorizationRequest::new(
            "registration-id",
            "https://example.com/login/oauth/authorize?response_type=code&client_id=client-id",
            "state",
            None,
            vec!["read:user".to_string()],
        )
        .set_grant_type(AuthorizationGrantType::Other("client_credentials".to_string()));
        let resolver = FixedRequestResolver {
            resolve_result: Ok(Some(authorization_request)),
            ..Default::default()
        };
        let mut filter = OAuth2AuthorizationRequestRedirectFilter::new(Arc::new(resolver));
        let recording_repository = RecordingAuthorizationRequestRepository::default();
        filter.set_authorization_request_repository(Arc::new(recording_repository.clone()));
        let chain = TestFilterChain::new(ChainOutcome::Continue);
        let mut request = get_request("/oauth2/authorization/registration-id");
        let mut response = TestResponse::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .unwrap();

        assert!(!chain.was_invoked());
        assert_eq!(response.status_code(), StatusCode::SEE_OTHER);
        assert!(recording_repository.saved.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn non_get_request_continues_chain() {
        let filter = redirect_filter();
        let chain = TestFilterChain::new(ChainOutcome::Continue);
        let mut request = post_request("/oauth2/authorization/registration-id");
        let mut response = TestResponse::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .unwrap();

        assert!(chain.was_invoked());
        assert!(response.header("location").is_none());
    }
}
