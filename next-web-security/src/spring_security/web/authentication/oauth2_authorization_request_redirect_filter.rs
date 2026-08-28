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
        let redirect_uri = authorization_request
            .authorization_request_uri()
            .to_string();
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
        request.set_attribute(
            &self.already_filtered_attribute_name,
            AnyValue::Boolean(true),
        );

        // Attempt to resolve the OAuth 2.0 Authorization Request.
        match self.authorization_request_resolver.resolve(request) {
            Ok(Some(authorization_request)) => {
                return self.send_redirect_for_authorization(
                    request,
                    response,
                    authorization_request,
                );
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
