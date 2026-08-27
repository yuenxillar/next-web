use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt,
    ops::{Deref, DerefMut},
    sync::Arc,
};

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
    authentication::{AuthenticationProvider, BaseAuthenticationBuilder, BaseAuthenticationToken},
    config::web::HttpSecurityBuilder,
    core::{
        Authentication, AuthenticationBuilder, AuthenticationError, AuthenticationErrorKind,
        GrantedAuthority, Principal,
    },
    oauth2::{
        request_parameter, ClientRegistration, ClientRegistrationRepository,
        InMemoryClientRegistrationRepository,
    },
    oauth2_resource_server::jwt::{Jwt, JwtDecoder, NimbusJwtDecoder},
    web::{
        authentication::{AuthPrincipal, AuthenticationConverter},
        authentication::logout::{CompositeLogoutHandler, LogoutHandler, SecurityContextLogoutHandler},
        csrf::CsrfFilter,
        util::matcher::RequestMatcher,
    },
};

use super::oidc_session_registry::{InMemoryOidcSessionRegistry, OidcSessionRegistry};

/// A decoded OIDC back-channel logout token. Mirrors Spring Security's
/// `OidcLogoutToken`, which is an `AbstractOAuth2Token` exposing the `iss`,
/// `aud`, `sub`, `sid`, and `jti` claims.
#[derive(Clone, Debug)]
pub struct OidcLogoutToken {
    token_value: String,
    claims: HashMap<String, String>,
}

impl OidcLogoutToken {
    /// Builds an `OidcLogoutToken` from a decoded `Jwt`.
    pub fn from_jwt(jwt: &Jwt) -> Self {
        Self {
            token_value: jwt.token_value().to_string(),
            claims: jwt.claims().clone(),
        }
    }

    pub fn token_value(&self) -> &str {
        &self.token_value
    }

    pub fn claims(&self) -> &HashMap<String, String> {
        &self.claims
    }

    pub fn issuer(&self) -> Option<&str> {
        self.claims.get("iss").map(String::as_str)
    }

    pub fn audience(&self) -> Vec<&str> {
        self.claims
            .get("aud")
            .map(|aud| {
                aud.split([' ', ','])
                    .filter(|value| !value.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn subject(&self) -> Option<&str> {
        self.claims.get("sub").map(String::as_str)
    }

    pub fn session_id(&self) -> Option<&str> {
        self.claims.get("sid").map(String::as_str)
    }

    pub fn jti(&self) -> Option<&str> {
        self.claims.get("jti").map(String::as_str)
    }
}

/// An `Authentication` representing an unauthenticated OIDC back-channel logout
/// request. Mirrors Spring Security's `OidcLogoutAuthenticationToken`.
#[derive(Clone)]
pub struct OidcLogoutAuthenticationToken {
    principal: Option<AuthPrincipal>,
    credentials: Option<AuthPrincipal>,
    logout_token: String,
    client_registration: ClientRegistration,
    base: BaseAuthenticationToken,
}

impl OidcLogoutAuthenticationToken {
    pub fn unauthenticated(
        logout_token: impl Into<String>,
        client_registration: ClientRegistration,
    ) -> Self {
        let logout_token = logout_token.into();
        let credentials = Arc::new(logout_token.clone()) as AuthPrincipal;
        let mut authentication = Self {
            principal: None,
            credentials: Some(credentials),
            logout_token,
            client_registration,
            base: BaseAuthenticationToken::new(None),
        };
        authentication
            .set_authenticated(false)
            .expect("OidcLogoutAuthenticationToken cannot be set trusted directly");
        authentication
    }

    pub fn authenticated(
        principal: AuthPrincipal,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
        logout_token: impl Into<String>,
        client_registration: ClientRegistration,
    ) -> Self {
        let mut inner = BaseAuthenticationToken::new(Some(authorities));
        inner
            .set_authenticated(true)
            .expect("authenticated OIDC logout token should accept trusted state");
        Self {
            principal: Some(principal),
            credentials: None,
            logout_token: logout_token.into(),
            client_registration,
            base: inner,
        }
    }

    pub fn logout_token(&self) -> &str {
        &self.logout_token
    }

    pub fn client_registration(&self) -> &ClientRegistration {
        &self.client_registration
    }

    fn from_builder(builder: &mut OidcLogoutAuthenticationTokenBuilder) -> Self {
        Self {
            principal: builder.principal.take(),
            credentials: builder.credentials.take(),
            logout_token: builder.logout_token.clone(),
            client_registration: builder.client_registration.clone(),
            base: BaseAuthenticationToken::from_builder(builder),
        }
    }
}

impl Authentication for OidcLogoutAuthenticationToken {
    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        self.base.authorities()
    }

    fn credentials(&self) -> Option<&AuthPrincipal> {
        self.credentials.as_ref()
    }

    fn details(&self) -> Option<&AuthPrincipal> {
        self.base.details()
    }

    fn principal(&self) -> Option<&AuthPrincipal> {
        self.principal.as_ref()
    }

    fn is_authenticated(&self) -> bool {
        self.base.is_authenticated()
    }

    fn set_authenticated(&mut self, is_authenticated: bool) -> Result<(), next_web_core::error::BoxError> {
        if is_authenticated {
            return Err("Cannot set this token to trusted - use authenticated()".into());
        }
        self.base.set_authenticated(false)
    }

    fn to_builder(&self) -> Box<dyn AuthenticationBuilder> {
        Box::new(OidcLogoutAuthenticationTokenBuilder::with_token(self))
    }

    fn of(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

impl Principal for OidcLogoutAuthenticationToken {
    fn name(&self) -> &str {
        self.principal
            .as_ref()
            .and_then(|principal| principal.downcast_ref::<String>())
            .map(String::as_str)
            .unwrap_or_else(|| self.client_registration.registration_id())
    }
}

impl fmt::Display for OidcLogoutAuthenticationToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} [Principal={}, Authenticated={}, ClientRegistration={}]",
            std::any::type_name::<Self>(),
            self.name(),
            self.is_authenticated(),
            self.client_registration.registration_id()
        )
    }
}

impl Deref for OidcLogoutAuthenticationToken {
    type Target = BaseAuthenticationToken;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for OidcLogoutAuthenticationToken {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

pub struct OidcLogoutAuthenticationTokenBuilder {
    principal: Option<AuthPrincipal>,
    credentials: Option<AuthPrincipal>,
    logout_token: String,
    client_registration: ClientRegistration,
    base: BaseAuthenticationBuilder,
}

impl OidcLogoutAuthenticationTokenBuilder {
    fn with_token(token: &OidcLogoutAuthenticationToken) -> Self {
        Self {
            principal: token.principal.clone(),
            credentials: token.credentials.clone(),
            logout_token: token.logout_token.clone(),
            client_registration: token.client_registration.clone(),
            base: BaseAuthenticationBuilder::with_token(token),
        }
    }
}

impl Deref for OidcLogoutAuthenticationTokenBuilder {
    type Target = BaseAuthenticationBuilder;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for OidcLogoutAuthenticationTokenBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl AuthenticationBuilder for OidcLogoutAuthenticationTokenBuilder {
    fn authorities(&mut self, authorities: Box<dyn FnOnce(&mut Vec<Arc<dyn GrantedAuthority>>)>) {
        self.base.authorities(authorities);
    }

    fn credentials(&mut self, credentials: Option<AuthPrincipal>) {
        self.credentials = credentials;
    }

    fn details(&mut self, details: Option<AuthPrincipal>) {
        self.base.details(details);
    }

    fn principal(&mut self, principal: Option<AuthPrincipal>) {
        self.principal = principal;
    }

    fn authenticated(&mut self, authenticated: bool) {
        self.base.authenticated(authenticated);
    }

    fn build(&mut self) -> Arc<dyn Authentication> {
        Arc::new(OidcLogoutAuthenticationToken::from_builder(self))
    }
}

/// The authenticated result of decoding an OIDC back-channel logout token.
/// Mirrors Spring Security's `OidcBackChannelLogoutAuthentication`.
#[derive(Clone)]
pub struct OidcBackChannelLogoutAuthentication {
    principal: Option<AuthPrincipal>,
    credentials: Option<AuthPrincipal>,
    logout_token: OidcLogoutToken,
    client_registration: ClientRegistration,
    base: BaseAuthenticationToken,
}

impl OidcBackChannelLogoutAuthentication {
    pub fn authenticated(
        logout_token: OidcLogoutToken,
        client_registration: ClientRegistration,
    ) -> Self {
        let mut inner = BaseAuthenticationToken::new(Some(Vec::new()));
        inner
            .set_authenticated(true)
            .expect("authenticated OIDC back-channel logout should accept trusted state");
        Self {
            principal: None,
            credentials: None,
            logout_token,
            client_registration,
            base: inner,
        }
    }

    pub fn logout_token(&self) -> &OidcLogoutToken {
        &self.logout_token
    }

    pub fn client_registration(&self) -> &ClientRegistration {
        &self.client_registration
    }

    fn from_builder(builder: &mut OidcBackChannelLogoutAuthenticationBuilder) -> Self {
        Self {
            principal: builder.principal.take(),
            credentials: builder.credentials.take(),
            logout_token: builder.logout_token.clone(),
            client_registration: builder.client_registration.clone(),
            base: BaseAuthenticationToken::from_builder(builder),
        }
    }
}

impl Authentication for OidcBackChannelLogoutAuthentication {
    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        self.base.authorities()
    }

    fn credentials(&self) -> Option<&AuthPrincipal> {
        self.credentials.as_ref()
    }

    fn details(&self) -> Option<&AuthPrincipal> {
        self.base.details()
    }

    fn principal(&self) -> Option<&AuthPrincipal> {
        self.principal.as_ref()
    }

    fn is_authenticated(&self) -> bool {
        self.base.is_authenticated()
    }

    fn set_authenticated(&mut self, is_authenticated: bool) -> Result<(), next_web_core::error::BoxError> {
        if is_authenticated {
            return Err("Cannot set this token to trusted - use authenticated()".into());
        }
        self.base.set_authenticated(false)
    }

    fn to_builder(&self) -> Box<dyn AuthenticationBuilder> {
        Box::new(OidcBackChannelLogoutAuthenticationBuilder::with_token(self))
    }

    fn of(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

impl Principal for OidcBackChannelLogoutAuthentication {
    fn name(&self) -> &str {
        self.client_registration.registration_id()
    }
}

impl fmt::Display for OidcBackChannelLogoutAuthentication {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} [Principal={}, Authenticated={}, ClientRegistration={}]",
            std::any::type_name::<Self>(),
            self.name(),
            self.is_authenticated(),
            self.client_registration.registration_id()
        )
    }
}

impl Deref for OidcBackChannelLogoutAuthentication {
    type Target = BaseAuthenticationToken;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for OidcBackChannelLogoutAuthentication {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

pub struct OidcBackChannelLogoutAuthenticationBuilder {
    principal: Option<AuthPrincipal>,
    credentials: Option<AuthPrincipal>,
    logout_token: OidcLogoutToken,
    client_registration: ClientRegistration,
    base: BaseAuthenticationBuilder,
}

impl OidcBackChannelLogoutAuthenticationBuilder {
    fn with_token(token: &OidcBackChannelLogoutAuthentication) -> Self {
        Self {
            principal: token.principal.clone(),
            credentials: token.credentials.clone(),
            logout_token: token.logout_token.clone(),
            client_registration: token.client_registration.clone(),
            base: BaseAuthenticationBuilder::with_token(token),
        }
    }
}

impl Deref for OidcBackChannelLogoutAuthenticationBuilder {
    type Target = BaseAuthenticationBuilder;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for OidcBackChannelLogoutAuthenticationBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl AuthenticationBuilder for OidcBackChannelLogoutAuthenticationBuilder {
    fn authorities(&mut self, authorities: Box<dyn FnOnce(&mut Vec<Arc<dyn GrantedAuthority>>)>) {
        self.base.authorities(authorities);
    }

    fn credentials(&mut self, credentials: Option<AuthPrincipal>) {
        self.credentials = credentials;
    }

    fn details(&mut self, details: Option<AuthPrincipal>) {
        self.base.details(details);
    }

    fn principal(&mut self, principal: Option<AuthPrincipal>) {
        self.principal = principal;
    }

    fn authenticated(&mut self, authenticated: bool) {
        self.base.authenticated(authenticated);
    }

    fn build(&mut self) -> Arc<dyn Authentication> {
        Arc::new(OidcBackChannelLogoutAuthentication::from_builder(self))
    }
}

/// Converts an OIDC back-channel logout HTTP request into an unauthenticated
/// `OidcLogoutAuthenticationToken`. Mirrors Spring Security's
/// `OidcLogoutAuthenticationConverter`.
#[derive(Clone)]
pub struct OidcLogoutAuthenticationConverter {
    client_registration_repository: Arc<dyn ClientRegistrationRepository>,
    request_matcher: Option<Arc<dyn RequestMatcher>>,
}

impl OidcLogoutAuthenticationConverter {
    pub fn new(client_registration_repository: Arc<dyn ClientRegistrationRepository>) -> Self {
        Self {
            client_registration_repository,
            request_matcher: None,
        }
    }

    pub fn set_request_matcher(&mut self, request_matcher: Arc<dyn RequestMatcher>) -> &mut Self {
        self.request_matcher = Some(request_matcher);
        self
    }
}

fn extract_registration_id(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() == 5
        && parts[1] == "logout"
        && parts[2] == "connect"
        && parts[3] == "back-channel"
    {
        let id = parts[4];
        if id.is_empty() {
            None
        } else {
            Some(id.to_string())
        }
    } else {
        None
    }
}

impl AuthenticationConverter for OidcLogoutAuthenticationConverter {
    fn convert(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<Box<dyn Authentication>>, AuthenticationError> {
        if request.method() != HttpMethod::POST {
            return Ok(None);
        }
        if let Some(matcher) = &self.request_matcher {
            if !matcher.matches(request) {
                return Ok(None);
            }
        } else if extract_registration_id(request.path()).is_none() {
            return Ok(None);
        }
        let registration_id = extract_registration_id(request.path()).ok_or_else(|| {
            AuthenticationError::with_kind(
                "Invalid OIDC back-channel logout request",
                AuthenticationErrorKind::BadCredentials,
            )
        })?;
        let client_registration = self
            .client_registration_repository
            .find_by_registration_id(&registration_id)
            .ok_or_else(|| {
                AuthenticationError::with_kind(
                    format!("Invalid Client Registration with Id: {}", registration_id),
                    AuthenticationErrorKind::InvalidClientRegistrationId,
                )
            })?;
        let logout_token = request_parameter(request, "logout_token").ok_or_else(|| {
            AuthenticationError::with_kind(
                "Missing logout_token request parameter",
                AuthenticationErrorKind::BadCredentials,
            )
        })?;
        Ok(Some(Box::new(
            OidcLogoutAuthenticationToken::unauthenticated(logout_token, client_registration),
        )))
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Authenticates an `OidcLogoutAuthenticationToken` by decoding and validating
/// the contained logout token. Mirrors Spring Security's
/// `OidcBackChannelLogoutAuthenticationProvider`.
#[derive(Clone)]
pub struct OidcBackChannelLogoutAuthenticationProvider {
    decoder: Arc<dyn JwtDecoder>,
}

impl OidcBackChannelLogoutAuthenticationProvider {
    pub fn new(decoder: Arc<dyn JwtDecoder>) -> Self {
        Self { decoder }
    }

    pub fn set_decoder(&mut self, decoder: Arc<dyn JwtDecoder>) -> &mut Self {
        self.decoder = decoder;
        self
    }
}

#[async_trait]
impl AuthenticationProvider for OidcBackChannelLogoutAuthenticationProvider {
    async fn authenticate(
        &self,
        authentication: &Arc<dyn Authentication>,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        if authentication.of() != TypeId::of::<OidcLogoutAuthenticationToken>() {
            return Ok(None);
        }
        let as_any: &dyn Any = authentication.as_ref();
        let token = as_any
            .downcast_ref::<OidcLogoutAuthenticationToken>()
            .ok_or_else(|| {
                AuthenticationError::with_kind(
                    "Authentication is not an OidcLogoutAuthenticationToken",
                    AuthenticationErrorKind::AuthenticationService,
                )
            })?;
        let jwt = self.decoder.decode(token.logout_token())?;
        let logout_token = OidcLogoutToken::from_jwt(&jwt);
        let authenticated = OidcBackChannelLogoutAuthentication::authenticated(
            logout_token,
            token.client_registration().clone(),
        );
        Ok(Some(Arc::new(authenticated)))
    }

    fn supports(&self, authentication: TypeId) -> bool {
        authentication == TypeId::of::<OidcLogoutAuthenticationToken>()
    }
}

/// A `LogoutHandler` that terminates each provider session associated with an
/// OIDC back-channel logout token. Mirrors Spring Security's
/// `OidcBackChannelLogoutHandler`; the per-session back-channel `POST` is not
/// performed in this port (no REST client is wired), but the session registry
/// entries are removed, which is the essential state change.
#[derive(Clone)]
pub struct OidcBackChannelLogoutHandler {
    session_registry: Arc<dyn OidcSessionRegistry>,
    logout_uri: String,
    session_cookie_name: String,
}

impl OidcBackChannelLogoutHandler {
    pub fn new(session_registry: Arc<dyn OidcSessionRegistry>) -> Self {
        Self {
            session_registry,
            logout_uri: "{baseUrl}/logout/connect/back-channel/{registrationId}".to_string(),
            session_cookie_name: "JSESSIONID".to_string(),
        }
    }

    pub fn with_logout_uri(
        session_registry: Arc<dyn OidcSessionRegistry>,
        logout_uri: impl Into<String>,
    ) -> Self {
        Self {
            session_registry,
            logout_uri: logout_uri.into(),
            session_cookie_name: "JSESSIONID".to_string(),
        }
    }

    pub fn set_logout_uri(&mut self, logout_uri: impl Into<String>) -> &mut Self {
        self.logout_uri = logout_uri.into();
        self
    }

    pub fn set_session_cookie_name(&mut self, session_cookie_name: impl Into<String>) -> &mut Self {
        self.session_cookie_name = session_cookie_name.into();
        self
    }

    pub fn logout_uri(&self) -> &str {
        &self.logout_uri
    }

    pub fn session_cookie_name(&self) -> &str {
        &self.session_cookie_name
    }
}

#[async_trait]
impl LogoutHandler for OidcBackChannelLogoutHandler {
    async fn logout(
        &self,
        _request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
        authentication: Option<&Arc<dyn Authentication>>,
    ) {
        let token = match authentication.and_then(|authentication| {
            let any: &dyn Any = authentication.as_ref();
            any.downcast_ref::<OidcBackChannelLogoutAuthentication>()
        }) {
            Some(token) => token,
            None => return,
        };
        let removed = self
            .session_registry
            .remove_session_information_by_token(token.logout_token());
        for session in removed {
            // TODO: perform the per-session back-channel logout POST to the
            // session's logout endpoint. A REST client is required; the registry
            // entry has already been removed above.
            tracing::warn!(
                "OIDC back-channel logout should POST to the session endpoint for {} (REST client not wired)",
                session.session_id()
            );
        }
    }
}

/// Delegates to one of two `LogoutHandler`s depending on whether the request
/// carries the internal logout marker. Mirrors Spring Security's nested
/// `EitherLogoutHandler`.
#[derive(Clone)]
pub struct EitherLogoutHandler {
    left: Arc<dyn LogoutHandler>,
    right: Arc<dyn LogoutHandler>,
}

impl EitherLogoutHandler {
    pub fn new(left: Arc<dyn LogoutHandler>, right: Arc<dyn LogoutHandler>) -> Self {
        Self { left, right }
    }
}

#[async_trait]
impl LogoutHandler for EitherLogoutHandler {
    async fn logout(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: Option<&Arc<dyn Authentication>>,
    ) {
        let internal = request
            .query()
            .map(|query| query.contains("_spring_security_internal_logout"))
            .unwrap_or(false);
        if internal {
            self.right.logout(request, response, authentication).await;
        } else {
            self.left.logout(request, response, authentication).await;
        }
    }
}

/// The filter that processes OIDC back-channel logout requests. Mirrors Spring
/// Security's `OidcBackChannelLogoutFilter`, which converts the request,
/// authenticates the logout token, and then invokes the `LogoutHandler`.
#[derive(Clone)]
pub struct OidcBackChannelLogoutFilter {
    authentication_converter: Arc<dyn AuthenticationConverter>,
    authentication_provider: Arc<dyn AuthenticationProvider>,
    logout_handler: Arc<dyn LogoutHandler>,
}

impl OidcBackChannelLogoutFilter {
    pub fn new(
        authentication_converter: Arc<dyn AuthenticationConverter>,
        authentication_provider: Arc<dyn AuthenticationProvider>,
        logout_handler: Arc<dyn LogoutHandler>,
    ) -> Self {
        Self {
            authentication_converter,
            authentication_provider,
            logout_handler,
        }
    }
}

const INTERNAL_LOGOUT_ERROR_BODY: &[u8] = br#"{"error":"invalid_logout_token"}"#;

#[async_trait]
impl HttpFilter for OidcBackChannelLogoutFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        let converted = match self.authentication_converter.convert(request) {
            Ok(Some(authentication)) => authentication,
            Ok(None) => return filter_chain.do_filter(request, response).await,
            Err(_) => {
                response.set_status_code(StatusCode::BAD_REQUEST);
                let _ = response.set_body(INTERNAL_LOGOUT_ERROR_BODY.to_vec());
                return Ok(());
            }
        };
        let authentication: Arc<dyn Authentication> = Arc::from(converted);
        let authenticated = match self.authentication_provider.authenticate(&authentication).await {
            Ok(Some(authenticated)) => authenticated,
            _ => {
                response.set_status_code(StatusCode::BAD_REQUEST);
                let _ = response.set_body(INTERNAL_LOGOUT_ERROR_BODY.to_vec());
                return Ok(());
            }
        };
        self.logout_handler
            .logout(request, response, Some(&authenticated))
            .await;
        filter_chain.do_filter(request, response).await
    }

    fn supports(&self, name: &str) -> bool {
        name == "OidcBackChannelLogoutFilter"
    }
}

impl Named for OidcBackChannelLogoutFilter {
    fn name(&self) -> &str {
        "OidcBackChannelLogoutFilter"
    }
}

/// Convenience helpers for resolving shared objects during OIDC logout
/// configuration. Mirrors Spring Security's `OAuth2ClientConfigurerUtils`.
pub struct OAuth2ClientConfigurerUtils;

impl OAuth2ClientConfigurerUtils {
    pub fn get_client_registration_repository<B>(
        http: &B,
    ) -> Arc<dyn ClientRegistrationRepository>
    where
        B: HttpSecurityBuilder<B> + 'static,
    {
        if let Some(repository) = http.shared_object::<Arc<dyn ClientRegistrationRepository>>() {
            return repository.clone();
        }
        Arc::new(InMemoryClientRegistrationRepository::default())
    }

    pub fn get_oidc_session_registry<B>(http: &mut B) -> Arc<dyn OidcSessionRegistry>
    where
        B: HttpSecurityBuilder<B> + 'static,
    {
        if let Some(registry) = http.shared_object::<Arc<dyn OidcSessionRegistry>>() {
            return registry.clone();
        }
        let registry: Arc<dyn OidcSessionRegistry> =
            Arc::new(InMemoryOidcSessionRegistry::default());
        http.set_shared_object(registry.clone());
        registry
    }
}

/// Brings the default session registry type into scope for callers.
pub use super::oidc_session_registry::InMemoryOidcSessionRegistry as DefaultOidcSessionRegistry;
