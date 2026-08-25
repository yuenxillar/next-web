//! Minimal Rust translation surface for Spring Security OAuth2 login.

use std::{
    any::TypeId,
    collections::HashMap,
    fmt,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use next_web_core::{
    anys::any_value::AnyValue,
    async_trait,
    filter::{FilterChainError, FilterError},
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use uuid::Uuid;

use crate::{
    authentication::{AuthenticationProvider, BaseAuthenticationBuilder, BaseAuthenticationToken},
    core::{
        authority_mapping::GrantedAuthoritiesMapper,
        Authentication, AuthenticationBuilder, GrantedAuthority, Principal,
        {AuthenticationError, AuthenticationErrorKind},
    },
    web::authentication::AuthPrincipal,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthorizationGrantType {
    AuthorizationCode,
    Other(String),
}

impl AuthorizationGrantType {
    pub fn authorization_code() -> Self {
        Self::AuthorizationCode
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientRegistration {
    registration_id: String,
    client_name: String,
    client_id: String,
    authorization_uri: String,
    redirect_uri: Option<String>,
    scopes: Vec<String>,
    authorization_grant_type: AuthorizationGrantType,
}

impl ClientRegistration {
    pub fn new(
        registration_id: impl Into<String>,
        client_name: impl Into<String>,
        authorization_uri: impl Into<String>,
    ) -> Self {
        let registration_id = registration_id.into();
        let client_name = client_name.into();
        let authorization_uri = authorization_uri.into();
        assert!(
            !registration_id.trim().is_empty(),
            "registration_id cannot be empty"
        );
        assert!(
            !client_name.trim().is_empty(),
            "client_name cannot be empty"
        );
        assert!(
            !authorization_uri.trim().is_empty(),
            "authorization_uri cannot be empty"
        );
        Self {
            registration_id,
            client_name,
            client_id: String::new(),
            authorization_uri,
            redirect_uri: None,
            scopes: Vec::new(),
            authorization_grant_type: AuthorizationGrantType::AuthorizationCode,
        }
    }

    pub fn set_client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = client_id.into();
        self
    }

    pub fn set_redirect_uri(mut self, redirect_uri: impl Into<String>) -> Self {
        self.redirect_uri = Some(redirect_uri.into());
        self
    }

    pub fn set_scopes(mut self, scopes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.scopes = scopes.into_iter().map(Into::into).collect();
        self
    }

    pub fn set_authorization_grant_type(
        mut self,
        authorization_grant_type: AuthorizationGrantType,
    ) -> Self {
        self.authorization_grant_type = authorization_grant_type;
        self
    }

    pub fn registration_id(&self) -> &str {
        &self.registration_id
    }

    pub fn client_name(&self) -> &str {
        &self.client_name
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn authorization_uri(&self) -> &str {
        &self.authorization_uri
    }

    pub fn redirect_uri(&self) -> Option<&str> {
        self.redirect_uri.as_deref()
    }

    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }

    pub fn authorization_grant_type(&self) -> &AuthorizationGrantType {
        &self.authorization_grant_type
    }

    pub fn is_authorization_code(&self) -> bool {
        self.authorization_grant_type == AuthorizationGrantType::AuthorizationCode
    }
}

pub trait ClientRegistrationRepository: Send + Sync {
    fn find_by_registration_id(&self, registration_id: &str) -> Option<ClientRegistration>;

    fn client_registrations(&self) -> Vec<ClientRegistration> {
        Vec::new()
    }
}

#[derive(Clone, Default)]
pub struct InMemoryClientRegistrationRepository {
    registrations: Vec<ClientRegistration>,
}

impl InMemoryClientRegistrationRepository {
    pub fn new(registrations: impl IntoIterator<Item = ClientRegistration>) -> Self {
        Self {
            registrations: registrations.into_iter().collect(),
        }
    }
}

impl ClientRegistrationRepository for InMemoryClientRegistrationRepository {
    fn find_by_registration_id(&self, registration_id: &str) -> Option<ClientRegistration> {
        self.registrations
            .iter()
            .find(|registration| registration.registration_id() == registration_id)
            .cloned()
    }

    fn client_registrations(&self) -> Vec<ClientRegistration> {
        self.registrations.clone()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OAuth2AuthorizationRequest {
    registration_id: String,
    authorization_request_uri: String,
    state: String,
    redirect_uri: Option<String>,
    scopes: Vec<String>,
    grant_type: AuthorizationGrantType,
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
        }
    }

    /// Builder-style setter for the authorization grant type.
    /// Defaults to `AuthorizationCode`, mirroring the Java builder.
    pub fn set_grant_type(mut self, grant_type: AuthorizationGrantType) -> Self {
        self.grant_type = grant_type;
        self
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

/// Indicates that an OAuth 2.0 Client is required to obtain authorization
/// from the Resource Owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientAuthorizationRequiredError {
    client_registration_id: String,
}

impl ClientAuthorizationRequiredError {
    pub fn new(client_registration_id: impl Into<String>) -> Self {
        Self {
            client_registration_id: client_registration_id.into(),
        }
    }

    pub fn client_registration_id(&self) -> &str {
        &self.client_registration_id
    }
}

impl fmt::Display for ClientAuthorizationRequiredError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Authorization required for Client Registration Id: {}",
            self.client_registration_id
        )
    }
}

impl std::error::Error for ClientAuthorizationRequiredError {}

impl From<ClientAuthorizationRequiredError> for FilterError {
    fn from(error: ClientAuthorizationRequiredError) -> Self {
        FilterError::Chain(FilterChainError::AnyError(Box::new(error)))
    }
}

pub trait OAuth2AuthorizationRequestResolver: Send + Sync {
    fn resolve(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<OAuth2AuthorizationRequest>, AuthenticationError>;

    fn resolve_with_registration_id(
        &self,
        request: &dyn HttpRequest,
        client_registration_id: &str,
    ) -> Result<Option<OAuth2AuthorizationRequest>, AuthenticationError>;
}

#[derive(Clone)]
pub struct DefaultOAuth2AuthorizationRequestResolver {
    client_registration_repository: Arc<dyn ClientRegistrationRepository>,
    authorization_request_base_uri: String,
}

impl DefaultOAuth2AuthorizationRequestResolver {
    pub fn new(
        client_registration_repository: Arc<dyn ClientRegistrationRepository>,
        authorization_request_base_uri: impl Into<String>,
    ) -> Self {
        let authorization_request_base_uri = authorization_request_base_uri.into();
        assert!(
            !authorization_request_base_uri.trim().is_empty(),
            "authorization_request_base_uri cannot be empty"
        );
        Self {
            client_registration_repository,
            authorization_request_base_uri,
        }
    }

    fn registration_id(&self, request: &dyn HttpRequest) -> Option<String> {
        extract_registration_id(request.path(), &self.authorization_request_base_uri)
    }

    fn authorization_request_for(
        &self,
        client_registration: ClientRegistration,
    ) -> OAuth2AuthorizationRequest {
        let state = Uuid::new_v4().to_string();
        let authorization_request_uri = authorization_request_uri(&client_registration, &state);
        OAuth2AuthorizationRequest::new(
            client_registration.registration_id().to_string(),
            authorization_request_uri,
            state,
            client_registration.redirect_uri().map(ToOwned::to_owned),
            client_registration.scopes().to_vec(),
        )
    }
}

impl OAuth2AuthorizationRequestResolver for DefaultOAuth2AuthorizationRequestResolver {
    fn resolve(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<OAuth2AuthorizationRequest>, AuthenticationError> {
        let Some(registration_id) = self.registration_id(request) else {
            // Not an authorization request URI: let the filter chain continue.
            return Ok(None);
        };
        self.resolve_with_registration_id(request, &registration_id)
    }

    fn resolve_with_registration_id(
        &self,
        _request: &dyn HttpRequest,
        client_registration_id: &str,
    ) -> Result<Option<OAuth2AuthorizationRequest>, AuthenticationError> {
        let registration = self
            .client_registration_repository
            .find_by_registration_id(client_registration_id)
            .ok_or_else(|| {
                AuthenticationError::with_kind(
                    format!("Invalid Client Registration with Id: {}", client_registration_id),
                    AuthenticationErrorKind::InvalidClientRegistrationId,
                )
            })?;
        // The Java resolver throws IllegalArgumentException for non-authorization-code
        // grants; returning Ok(None) is a documented simplification of this port.
        if !registration.is_authorization_code() {
            return Ok(None);
        }
        Ok(Some(self.authorization_request_for(registration)))
    }
}

pub trait AuthorizationRequestRepository: Send + Sync {
    fn load_authorization_request(
        &self,
        request: &dyn HttpRequest,
    ) -> Option<OAuth2AuthorizationRequest>;

    fn save_authorization_request(
        &self,
        authorization_request: Option<OAuth2AuthorizationRequest>,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    );

    fn remove_authorization_request(
        &self,
        request: &dyn HttpRequest,
    ) -> Option<OAuth2AuthorizationRequest>;
}

#[derive(Clone)]
pub struct HttpSessionOAuth2AuthorizationRequestRepository {
    session_attribute_name: String,
}

impl HttpSessionOAuth2AuthorizationRequestRepository {
    pub const DEFAULT_AUTHORIZATION_REQUEST_ATTR_NAME: &'static str =
        "next_web_security.oauth2.authorization_request";

    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for HttpSessionOAuth2AuthorizationRequestRepository {
    fn default() -> Self {
        Self {
            session_attribute_name: Self::DEFAULT_AUTHORIZATION_REQUEST_ATTR_NAME.to_string(),
        }
    }
}

impl AuthorizationRequestRepository for HttpSessionOAuth2AuthorizationRequestRepository {
    fn load_authorization_request(
        &self,
        request: &dyn HttpRequest,
    ) -> Option<OAuth2AuthorizationRequest> {
        request.session().and_then(|session| {
            session
                .attribute(&self.session_attribute_name)
                .and_then(|value| value.as_ref_object::<OAuth2AuthorizationRequest>())
                .cloned()
        })
    }

    fn save_authorization_request(
        &self,
        authorization_request: Option<OAuth2AuthorizationRequest>,
        request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
    ) {
        match authorization_request {
            Some(authorization_request) => {
                if let Some(session) = request.session_mut(true) {
                    session.set_attribute(
                        &self.session_attribute_name,
                        AnyValue::Object(Box::new(authorization_request)),
                    );
                }
            }
            None => {
                if let Some(session) = request.session() {
                    session.remove_attribute(&self.session_attribute_name);
                }
            }
        }
    }

    fn remove_authorization_request(
        &self,
        request: &dyn HttpRequest,
    ) -> Option<OAuth2AuthorizationRequest> {
        let session = request.session()?;
        let authorization_request = session
            .attribute(&self.session_attribute_name)
            .and_then(|value| value.as_ref_object::<OAuth2AuthorizationRequest>())
            .cloned();
        if authorization_request.is_some() {
            session.remove_attribute(&self.session_attribute_name);
        }
        authorization_request
    }
}

pub trait OAuth2AuthorizedClientService: Send + Sync {}

pub trait OAuth2AuthorizedClientRepository: Send + Sync {}

#[derive(Clone)]
pub struct AuthenticatedPrincipalOAuth2AuthorizedClientRepository {
    authorized_client_service: Arc<dyn OAuth2AuthorizedClientService>,
}

impl AuthenticatedPrincipalOAuth2AuthorizedClientRepository {
    pub fn new(authorized_client_service: Arc<dyn OAuth2AuthorizedClientService>) -> Self {
        Self {
            authorized_client_service,
        }
    }

    pub fn authorized_client_service(&self) -> &Arc<dyn OAuth2AuthorizedClientService> {
        &self.authorized_client_service
    }
}

impl OAuth2AuthorizedClientRepository for AuthenticatedPrincipalOAuth2AuthorizedClientRepository {}

#[derive(Clone, Default)]
pub struct NullOAuth2AuthorizedClientService;

impl OAuth2AuthorizedClientService for NullOAuth2AuthorizedClientService {}

#[derive(Clone)]
pub struct OAuth2AuthorizationCodeGrantRequest {
    client_registration: ClientRegistration,
    authorization_exchange: OAuth2AuthorizationExchange,
}

impl OAuth2AuthorizationCodeGrantRequest {
    pub fn new(
        client_registration: ClientRegistration,
        authorization_exchange: OAuth2AuthorizationExchange,
    ) -> Self {
        Self {
            client_registration,
            authorization_exchange,
        }
    }

    pub fn client_registration(&self) -> &ClientRegistration {
        &self.client_registration
    }

    pub fn authorization_exchange(&self) -> &OAuth2AuthorizationExchange {
        &self.authorization_exchange
    }
}

#[derive(Clone, Debug, Default)]
pub struct OAuth2AccessTokenResponse {
    access_token: String,
}

impl OAuth2AccessTokenResponse {
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
        }
    }

    pub fn access_token(&self) -> &str {
        &self.access_token
    }
}

pub trait OAuth2AccessTokenResponseClient<R>: Send + Sync {
    fn get_token_response(
        &self,
        request: &R,
    ) -> Result<OAuth2AccessTokenResponse, AuthenticationError>;
}

#[derive(Clone, Default)]
pub struct RestClientAuthorizationCodeTokenResponseClient;

impl OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>
    for RestClientAuthorizationCodeTokenResponseClient
{
    fn get_token_response(
        &self,
        _request: &OAuth2AuthorizationCodeGrantRequest,
    ) -> Result<OAuth2AccessTokenResponse, AuthenticationError> {
        Err(AuthenticationError::with_kind(
            "OAuth2 token endpoint exchange is not implemented",
            AuthenticationErrorKind::AuthenticationService,
        ))
    }
}

#[derive(Clone)]
pub struct OAuth2UserRequest {
    client_registration: ClientRegistration,
    access_token_response: OAuth2AccessTokenResponse,
}

impl OAuth2UserRequest {
    pub fn new(
        client_registration: ClientRegistration,
        access_token_response: OAuth2AccessTokenResponse,
    ) -> Self {
        Self {
            client_registration,
            access_token_response,
        }
    }
}

#[derive(Clone, Default)]
pub struct OAuth2User {
    name: String,
    attributes: HashMap<String, String>,
}

impl OAuth2User {
    pub fn new(name: impl Into<String>, attributes: HashMap<String, String>) -> Self {
        Self {
            name: name.into(),
            attributes,
        }
    }
}

pub trait OAuth2UserService<R, U>: Send + Sync {
    fn load_user(&self, request: &R) -> Result<U, AuthenticationError>;
}

#[derive(Clone, Default)]
pub struct DefaultOAuth2UserService;

impl OAuth2UserService<OAuth2UserRequest, OAuth2User> for DefaultOAuth2UserService {
    fn load_user(&self, _request: &OAuth2UserRequest) -> Result<OAuth2User, AuthenticationError> {
        Err(AuthenticationError::with_kind(
            "OAuth2 UserInfo endpoint lookup is not implemented",
            AuthenticationErrorKind::AuthenticationService,
        ))
    }
}

#[derive(Clone)]
pub struct OidcUserRequest {
    client_registration: ClientRegistration,
    access_token_response: OAuth2AccessTokenResponse,
}

impl OidcUserRequest {
    pub fn new(
        client_registration: ClientRegistration,
        access_token_response: OAuth2AccessTokenResponse,
    ) -> Self {
        Self {
            client_registration,
            access_token_response,
        }
    }
}

#[derive(Clone, Default)]
pub struct OidcUser {
    name: String,
    issuer: Option<String>,
}

#[derive(Clone, Default)]
pub struct OidcUserService;

impl OAuth2UserService<OidcUserRequest, OidcUser> for OidcUserService {
    fn load_user(&self, _request: &OidcUserRequest) -> Result<OidcUser, AuthenticationError> {
        Err(AuthenticationError::with_kind(
            "OIDC UserInfo endpoint lookup is not implemented",
            AuthenticationErrorKind::AuthenticationService,
        ))
    }
}

pub struct OAuth2LoginAuthenticationToken {
    principal: Option<AuthPrincipal>,
    credentials: Option<AuthPrincipal>,
    client_registration: ClientRegistration,
    authorization_exchange: OAuth2AuthorizationExchange,
    cleared: AtomicBool,
    inner: BaseAuthenticationToken,
}

impl OAuth2LoginAuthenticationToken {
    pub fn unauthenticated(
        client_registration: ClientRegistration,
        authorization_exchange: OAuth2AuthorizationExchange,
    ) -> Self {
        let credentials = Arc::new(authorization_exchange.clone()) as AuthPrincipal;
        let mut token = Self {
            principal: None,
            credentials: Some(credentials),
            client_registration,
            authorization_exchange,
            cleared: AtomicBool::new(false),
            inner: BaseAuthenticationToken::new(None),
        };
        token
            .set_authenticated(false)
            .expect("OAuth2LoginAuthenticationToken cannot be set trusted directly");
        token
    }

    pub fn authenticated(
        principal: AuthPrincipal,
        client_registration: ClientRegistration,
        authorization_exchange: OAuth2AuthorizationExchange,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        let mut inner = BaseAuthenticationToken::new(Some(authorities));
        inner
            .set_authenticated(true)
            .expect("authenticated OAuth2 token should accept trusted state");
        Self {
            principal: Some(principal),
            credentials: None,
            client_registration,
            authorization_exchange,
            cleared: AtomicBool::new(false),
            inner,
        }
    }

    pub fn client_registration(&self) -> &ClientRegistration {
        &self.client_registration
    }

    pub fn authorization_exchange(&self) -> &OAuth2AuthorizationExchange {
        &self.authorization_exchange
    }

    fn from_builder(builder: &mut OAuth2LoginAuthenticationTokenBuilder) -> Self {
        Self {
            principal: builder.principal.take(),
            credentials: builder.credentials.take(),
            client_registration: builder.client_registration.clone(),
            authorization_exchange: builder.authorization_exchange.clone(),
            cleared: AtomicBool::new(false),
            inner: BaseAuthenticationToken::from_builder(builder),
        }
    }
}

impl Clone for OAuth2LoginAuthenticationToken {
    fn clone(&self) -> Self {
        Self {
            principal: self.principal.clone(),
            credentials: self.credentials.clone(),
            client_registration: self.client_registration.clone(),
            authorization_exchange: self.authorization_exchange.clone(),
            cleared: AtomicBool::new(self.cleared.load(Ordering::Acquire)),
            inner: self.inner.clone(),
        }
    }
}

impl Authentication for OAuth2LoginAuthenticationToken {
    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        self.inner.authorities()
    }

    fn credentials(&self) -> Option<&AuthPrincipal> {
        if self.cleared.load(Ordering::Acquire) {
            return None;
        }
        self.credentials.as_ref()
    }

    fn details(&self) -> Option<&AuthPrincipal> {
        self.inner.details()
    }

    fn principal(&self) -> Option<&AuthPrincipal> {
        self.principal.as_ref()
    }

    fn is_authenticated(&self) -> bool {
        self.inner.is_authenticated()
    }

    fn set_authenticated(
        &mut self,
        is_authenticated: bool,
    ) -> Result<(), next_web_core::error::BoxError> {
        if is_authenticated {
            return Err("Cannot set this token to trusted - use authenticated()".into());
        }
        self.inner.set_authenticated(false)
    }

    fn to_builder(&self) -> Box<dyn AuthenticationBuilder> {
        Box::new(OAuth2LoginAuthenticationTokenBuilder::with_token(self))
    }

    fn of(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

impl Principal for OAuth2LoginAuthenticationToken {
    fn name(&self) -> &str {
        self.principal
            .as_ref()
            .and_then(|principal| principal.downcast_ref::<String>().map(String::as_str))
            .unwrap_or_else(|| self.client_registration.registration_id())
    }
}

impl fmt::Display for OAuth2LoginAuthenticationToken {
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

pub struct OAuth2LoginAuthenticationTokenBuilder {
    principal: Option<AuthPrincipal>,
    credentials: Option<AuthPrincipal>,
    client_registration: ClientRegistration,
    authorization_exchange: OAuth2AuthorizationExchange,
    inner: BaseAuthenticationBuilder,
}

impl OAuth2LoginAuthenticationTokenBuilder {
    fn with_token(token: &OAuth2LoginAuthenticationToken) -> Self {
        Self {
            principal: token.principal.clone(),
            credentials: token.credentials.clone(),
            client_registration: token.client_registration.clone(),
            authorization_exchange: token.authorization_exchange.clone(),
            inner: BaseAuthenticationBuilder::with_token(token),
        }
    }
}

impl AuthenticationBuilder for OAuth2LoginAuthenticationTokenBuilder {
    fn authorities(&mut self, authorities: Box<dyn FnOnce(&mut Vec<Arc<dyn GrantedAuthority>>)>) {
        self.inner.authorities(authorities);
    }

    fn credentials(&mut self, credentials: Option<AuthPrincipal>) {
        self.credentials = credentials;
    }

    fn details(&mut self, details: Option<AuthPrincipal>) {
        self.inner.details(details);
    }

    fn principal(&mut self, principal: Option<AuthPrincipal>) {
        self.principal = principal;
    }

    fn authenticated(&mut self, authenticated: bool) {
        self.inner.authenticated(authenticated);
    }

    fn build(&mut self) -> Arc<dyn Authentication> {
        Arc::new(OAuth2LoginAuthenticationToken::from_builder(self))
    }
}

impl std::ops::Deref for OAuth2LoginAuthenticationToken {
    type Target = BaseAuthenticationToken;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::ops::DerefMut for OAuth2LoginAuthenticationToken {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl std::ops::Deref for OAuth2LoginAuthenticationTokenBuilder {
    type Target = BaseAuthenticationBuilder;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::ops::DerefMut for OAuth2LoginAuthenticationTokenBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[derive(Clone)]
pub struct OAuth2LoginAuthenticationProvider {
    access_token_response_client:
        Arc<dyn OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>>,
    user_service: Arc<dyn OAuth2UserService<OAuth2UserRequest, OAuth2User>>,
    authorities_mapper: Option<Arc<dyn GrantedAuthoritiesMapper>>,
}

impl OAuth2LoginAuthenticationProvider {
    pub fn new(
        access_token_response_client: Arc<
            dyn OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>,
        >,
        user_service: Arc<dyn OAuth2UserService<OAuth2UserRequest, OAuth2User>>,
    ) -> Self {
        Self {
            access_token_response_client,
            user_service,
            authorities_mapper: None,
        }
    }

    pub fn set_authorities_mapper(
        &mut self,
        authorities_mapper: Arc<dyn GrantedAuthoritiesMapper>,
    ) {
        self.authorities_mapper = Some(authorities_mapper);
    }
}

#[async_trait]
impl AuthenticationProvider for OAuth2LoginAuthenticationProvider {
    async fn authenticate(
        &self,
        authentication: &Arc<dyn Authentication>,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        if authentication.of() != TypeId::of::<OAuth2LoginAuthenticationToken>() {
            return Ok(None);
        }
        let exchange = authentication
            .credentials()
            .and_then(|credentials| credentials.downcast_ref::<OAuth2AuthorizationExchange>())
            .ok_or_else(|| {
                AuthenticationError::with_kind(
                    "OAuth2 authorization exchange is missing",
                    AuthenticationErrorKind::AuthenticationService,
                )
            })?;
        if let Some(error_code) = exchange.authorization_response().error_code() {
            return Err(AuthenticationError::with_kind(
                format!(
                    "OAuth2 authorization response contained error: {}",
                    error_code
                ),
                AuthenticationErrorKind::BadCredentials,
            ));
        }
        Err(AuthenticationError::with_kind(
            "OAuth2 login authentication requires token endpoint and userinfo support, which are not implemented",
            AuthenticationErrorKind::AuthenticationService,
        ))
    }

    fn supports(&self, authentication: TypeId) -> bool {
        authentication == TypeId::of::<OAuth2LoginAuthenticationToken>()
    }
}

#[derive(Clone, Default)]
pub struct OidcAuthenticationRequestChecker;

#[async_trait]
impl AuthenticationProvider for OidcAuthenticationRequestChecker {
    async fn authenticate(
        &self,
        authentication: &Arc<dyn Authentication>,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        if authentication.of() != TypeId::of::<OAuth2LoginAuthenticationToken>() {
            return Ok(None);
        }
        let exchange = match authentication
            .credentials()
            .and_then(|credentials| credentials.downcast_ref::<OAuth2AuthorizationExchange>())
        {
            Some(exchange) => exchange,
            None => return Ok(None),
        };
        if exchange
            .authorization_request()
            .scopes()
            .iter()
            .any(|scope| scope == "openid")
        {
            return Err(AuthenticationError::with_kind(
                "An OpenID Connect Authentication Provider has not been configured. Check to ensure OIDC/JWT support is available.",
                AuthenticationErrorKind::AuthenticationService,
            ));
        }
        Ok(None)
    }

    fn supports(&self, authentication: TypeId) -> bool {
        authentication == TypeId::of::<OAuth2LoginAuthenticationToken>()
    }
}

pub fn request_parameter(request: &dyn HttpRequest, name: &str) -> Option<String> {
    request.query()?.split('&').find_map(|pair| {
        let mut parts = pair.splitn(2, '=');
        let key = decode(parts.next().unwrap_or_default());
        if key != name {
            return None;
        }
        Some(decode(parts.next().unwrap_or_default()))
    })
}

pub fn extract_registration_id(path: &str, authorization_request_base_uri: &str) -> Option<String> {
    let prefix = format!("{}/", authorization_request_base_uri.trim_end_matches('/'));
    let start = path.rfind(&prefix)?;
    let registration_id = &path[start + prefix.len()..];
    let registration_id = registration_id.split('/').next().unwrap_or_default();
    if registration_id.is_empty() {
        None
    } else {
        Some(registration_id.to_string())
    }
}

fn authorization_request_uri(client_registration: &ClientRegistration, state: &str) -> String {
    let mut params = Vec::new();
    params.push(("response_type", "code".to_string()));
    if !client_registration.client_id().is_empty() {
        params.push(("client_id", client_registration.client_id().to_string()));
    }
    if !client_registration.scopes().is_empty() {
        params.push(("scope", client_registration.scopes().join(" ")));
    }
    if let Some(redirect_uri) = client_registration.redirect_uri() {
        params.push(("redirect_uri", redirect_uri.to_string()));
    }
    params.push(("state", state.to_string()));

    let separator = if client_registration.authorization_uri().contains('?') {
        '&'
    } else {
        '?'
    };
    let query = params
        .into_iter()
        .map(|(key, value)| format!("{}={}", key, urlencoding::encode(&value)))
        .collect::<Vec<_>>()
        .join("&");

    format!(
        "{}{}{}",
        client_registration.authorization_uri(),
        separator,
        query
    )
}

fn decode(value: &str) -> String {
    urlencoding::decode(value)
        .map(|value| value.into_owned())
        .unwrap_or_else(|_| value.to_string())
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request as AxumRequest};

    use super::*;

    fn client_registration(
        registration_id: &str,
        grant_type: AuthorizationGrantType,
    ) -> ClientRegistration {
        ClientRegistration::new(
            registration_id,
            "Test Client",
            "https://example.com/login/oauth/authorize",
        )
        .set_client_id("client-id")
        .set_authorization_grant_type(grant_type)
    }

    fn resolver() -> DefaultOAuth2AuthorizationRequestResolver {
        DefaultOAuth2AuthorizationRequestResolver::new(
            Arc::new(InMemoryClientRegistrationRepository::new([
                client_registration("registration-id", AuthorizationGrantType::AuthorizationCode),
                client_registration(
                    "client-credentials",
                    AuthorizationGrantType::Other("client_credentials".to_string()),
                ),
            ])),
            "/oauth2/authorization",
        )
    }

    fn get_request(path: &str) -> AxumRequest {
        let mut request = AxumRequest::builder()
            .uri(path)
            .body(Body::empty())
            .unwrap();
        request.ready();
        request
    }

    #[test]
    fn resolve_returns_none_when_not_authorization_request_uri() {
        let resolver = resolver();
        let request = get_request("/path");
        assert!(resolver.resolve(&request).unwrap().is_none());
    }

    #[test]
    fn resolve_errors_when_registration_id_is_unknown() {
        let resolver = resolver();
        let request = get_request("/oauth2/authorization/missing");
        let error = resolver.resolve(&request).unwrap_err();
        assert_eq!(
            error.kind(),
            AuthenticationErrorKind::InvalidClientRegistrationId
        );
        assert_eq!(
            error.message(),
            "Invalid Client Registration with Id: missing"
        );
    }

    #[test]
    fn resolve_returns_authorization_request_for_authorization_code_client() {
        let resolver = resolver();
        let request = get_request("/oauth2/authorization/registration-id");
        let authorization_request = resolver.resolve(&request).unwrap().unwrap();
        assert_eq!(authorization_request.registration_id(), "registration-id");
        assert_eq!(
            authorization_request.grant_type(),
            &AuthorizationGrantType::AuthorizationCode
        );
    }

    #[test]
    fn resolve_returns_none_for_non_authorization_code_client() {
        let resolver = resolver();
        let request = get_request("/oauth2/authorization/client-credentials");
        assert!(resolver.resolve(&request).unwrap().is_none());
    }

    #[test]
    fn resolve_with_registration_id_uses_provided_id() {
        let resolver = resolver();
        // The request path is ignored by resolve_with_registration_id.
        let request = get_request("/path");
        let authorization_request = resolver
            .resolve_with_registration_id(&request, "registration-id")
            .unwrap()
            .unwrap();
        assert_eq!(authorization_request.registration_id(), "registration-id");
    }
}
