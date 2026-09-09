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

    fn matches(&self, request: &dyn HttpRequest) -> bool {
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
        response.finish();
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
        if request.method() != HttpMethod::POST {
            return filter_chain.do_filter(request, response).await;
        }
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
