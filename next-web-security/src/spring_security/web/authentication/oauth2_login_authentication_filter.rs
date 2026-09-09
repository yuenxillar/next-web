use std::{
    fmt,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{
    async_trait,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use crate::{
    core::{
        context::SecurityContextHolderStrategy,
        Authentication, {AuthenticationError, AuthenticationErrorKind},
    },
    oauth2::{
        request_parameter, AuthorizationRequestRepository, ClientRegistrationRepository,
        HttpSessionOAuth2AuthorizationRequestRepository, OAuth2AuthorizationExchange,
        OAuth2AuthorizationResponse, OAuth2AuthorizedClient, OAuth2AuthorizedClientRepository,
        OAuth2LoginAuthenticationToken,
    },
    web::{
        authentication::{
            AuthenticationConverter, BaseAuthenticationProcessingFilter,
            BaseAuthenticationProcessingFilterExt,
        },
        context::SecurityContextRepository,
        util::matcher::RequestMatcher,
    },
};

/// Processes the OAuth 2.0 authorization-code callback.
///
/// The converter restores and validates the original authorization request,
/// the authentication manager exchanges the code and loads the user, and this
/// filter persists the resulting authorized client before the base filter
/// stores the authenticated principal in the security context.
#[derive(Clone)]
pub struct OAuth2LoginAuthenticationFilter {
    authorization_request_repository: Arc<dyn AuthorizationRequestRepository>,
    client_registration_repository: Arc<dyn ClientRegistrationRepository>,
    authorized_client_repository: Arc<dyn OAuth2AuthorizedClientRepository>,
    login_processing_url: String,
    base: BaseAuthenticationProcessingFilter,
}

impl OAuth2LoginAuthenticationFilter {
    pub const DEFAULT_FILTER_PROCESSES_URI: &'static str = "/login/oauth2/code/*";

    pub fn new(
        client_registration_repository: Arc<dyn ClientRegistrationRepository>,
        authorized_client_repository: Arc<dyn OAuth2AuthorizedClientRepository>,
        login_processing_url: impl Into<String>,
    ) -> Self {
        let login_processing_url = login_processing_url.into();
        let authorization_request_repository =
            Arc::new(HttpSessionOAuth2AuthorizationRequestRepository::default())
                as Arc<dyn AuthorizationRequestRepository>;
        let converter = Arc::new(OAuth2LoginAuthenticationConverter::new(
            authorization_request_repository.clone(),
            client_registration_repository.clone(),
        ));
        let mut base = BaseAuthenticationProcessingFilter::with_request_matcher(Arc::new(
            OAuth2LoginRequestMatcher::new(login_processing_url.clone()),
        ));
        base.set_authentication_converter(converter);

        Self {
            authorization_request_repository,
            client_registration_repository,
            authorized_client_repository,
            login_processing_url,
            base,
        }
    }

    pub fn set_authorization_request_repository(
        &mut self,
        authorization_request_repository: Arc<dyn AuthorizationRequestRepository>,
    ) {
        self.authorization_request_repository = authorization_request_repository.clone();
        self.base
            .set_authentication_converter(Arc::new(OAuth2LoginAuthenticationConverter::new(
                authorization_request_repository,
                self.client_registration_repository.clone(),
            )));
    }

    pub fn set_requires_authentication_request_matcher(
        &mut self,
        request_matcher: Arc<dyn RequestMatcher>,
    ) {
        self.base
            .set_requires_authentication_request_matcher(request_matcher);
    }

    pub fn set_filter_processes_url(&mut self, login_processing_url: impl Into<String>) {
        self.login_processing_url = login_processing_url.into();
        self.base
            .set_requires_authentication_request_matcher(Arc::new(OAuth2LoginRequestMatcher::new(
                self.login_processing_url.clone(),
            )));
    }

    pub fn set_security_context_repository(
        &mut self,
        security_context_repository: Arc<dyn SecurityContextRepository>,
    ) {
        self.base
            .set_security_context_repository(security_context_repository);
    }

    pub fn set_security_context_holder_strategy(
        &mut self,
        security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.base
            .set_security_context_holder_strategy(security_context_holder_strategy);
    }

    pub fn login_processing_url(&self) -> &str {
        &self.login_processing_url
    }
}

impl Deref for OAuth2LoginAuthenticationFilter {
    type Target = BaseAuthenticationProcessingFilter;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for OAuth2LoginAuthenticationFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

#[async_trait]
impl HttpFilter for OAuth2LoginAuthenticationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        BaseAuthenticationProcessingFilter::do_filter(request, response, filter_chain, self).await
    }
}

#[async_trait]
impl BaseAuthenticationProcessingFilterExt for OAuth2LoginAuthenticationFilter {
    async fn attempt_authentication(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        // Delegate token conversion and AuthenticationManager invocation to
        // the common browser-authentication pipeline.
        let authentication = self.base._attempt_authentication(request, response).await?;
        let Some(authentication) = authentication else {
            return Ok(None);
        };
        let login = (authentication.as_ref() as &dyn std::any::Any)
            .downcast_ref::<OAuth2LoginAuthenticationToken>()
            .ok_or_else(|| {
                AuthenticationError::with_kind(
                    "AuthenticationManager returned an unexpected OAuth2 authentication type",
                    AuthenticationErrorKind::AuthenticationService,
                )
            })?;
        let access_token = login.access_token().cloned().ok_or_else(|| {
            AuthenticationError::with_kind(
                "OAuth2 login authentication result did not contain an access token",
                AuthenticationErrorKind::AuthenticationService,
            )
        })?;
        // Spring associates the issued access token with the authenticated
        // principal before successful_authentication stores the principal in
        // SecurityContextRepository.
        let authorized_client = OAuth2AuthorizedClient::new(
            login.client_registration().clone(),
            authentication.name().into_owned(),
            access_token,
        );
        self.authorized_client_repository.save_authorized_client(
            authorized_client,
            authentication.as_ref(),
            request,
            response,
        );
        Ok(Some(authentication))
    }
}

impl Named for OAuth2LoginAuthenticationFilter {
    fn name(&self) -> &str {
        "OAuth2LoginAuthenticationFilter"
    }
}

#[derive(Clone)]
struct OAuth2LoginAuthenticationConverter {
    authorization_request_repository: Arc<dyn AuthorizationRequestRepository>,
    client_registration_repository: Arc<dyn ClientRegistrationRepository>,
}

impl OAuth2LoginAuthenticationConverter {
    fn new(
        authorization_request_repository: Arc<dyn AuthorizationRequestRepository>,
        client_registration_repository: Arc<dyn ClientRegistrationRepository>,
    ) -> Self {
        Self {
            authorization_request_repository,
            client_registration_repository,
        }
    }
}

impl AuthenticationConverter for OAuth2LoginAuthenticationConverter {
    fn convert(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<Box<dyn Authentication>>, AuthenticationError> {
        if let Some(error) = request_parameter(request, "error") {
            let description = request_parameter(request, "error_description")
                .map(|description| format!(": {}", description))
                .unwrap_or_default();
            return Err(AuthenticationError::with_kind(
                format!("OAuth2 authorization response contained error: {error}{description}"),
                AuthenticationErrorKind::BadCredentials,
            ));
        }

        let code = request_parameter(request, "code").ok_or_else(|| {
            AuthenticationError::with_kind(
                "OAuth2 authorization response did not contain a code",
                AuthenticationErrorKind::BadCredentials,
            )
        })?;
        let state = request_parameter(request, "state");
        let authorization_request = self
            .authorization_request_repository
            .remove_authorization_request(request)
            .ok_or_else(|| {
                AuthenticationError::with_kind(
                    "OAuth2 authorization request was not found",
                    AuthenticationErrorKind::BadCredentials,
                )
            })?;

        if authorization_request.state().is_empty()
            || state.as_deref() != Some(authorization_request.state())
        {
            return Err(AuthenticationError::with_kind(
                "OAuth2 authorization response state did not match the authorization request",
                AuthenticationErrorKind::BadCredentials,
            ));
        }

        let client_registration = self
            .client_registration_repository
            .find_by_registration_id(authorization_request.registration_id())
            .ok_or_else(|| {
                AuthenticationError::with_kind(
                    "OAuth2 client registration was not found",
                    AuthenticationErrorKind::BadCredentials,
                )
            })?;

        let authorization_response = OAuth2AuthorizationResponse::success(code, state);
        let authorization_exchange =
            OAuth2AuthorizationExchange::new(authorization_request, authorization_response);
        Ok(Some(Box::new(
            OAuth2LoginAuthenticationToken::unauthenticated(
                client_registration,
                authorization_exchange,
            ),
        )))
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Clone, Debug)]
pub struct OAuth2LoginRequestMatcher {
    pattern: String,
}

impl OAuth2LoginRequestMatcher {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
        }
    }
}

impl RequestMatcher for OAuth2LoginRequestMatcher {
    fn matches(&self, request: &dyn HttpRequest) -> bool {
        path_matches(&self.pattern, request.path())
    }
}

fn path_matches(pattern: &str, path: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    if let Some(prefix) = pattern.strip_suffix("/*") {
        return path
            .strip_prefix(prefix)
            .and_then(|remaining| remaining.strip_prefix('/'))
            .is_some_and(|segment| !segment.is_empty() && !segment.contains('/'));
    }
    path == pattern
}

impl fmt::Display for OAuth2LoginRequestMatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OAuth2LoginRequestMatcher [{}]", self.pattern)
    }
}

#[cfg(test)]
mod tests {
    use super::path_matches;

    #[test]
    fn single_wildcard_matches_exactly_one_registration_segment() {
        assert!(path_matches(
            "/login/oauth2/code/*",
            "/login/oauth2/code/github"
        ));
        assert!(!path_matches("/login/oauth2/code/*", "/login/oauth2/code"));
        assert!(!path_matches(
            "/login/oauth2/code/*",
            "/login/oauth2/code/github/extra"
        ));
    }
}
