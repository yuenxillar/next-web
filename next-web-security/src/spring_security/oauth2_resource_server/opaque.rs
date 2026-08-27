use std::{any::{Any, TypeId}, collections::HashMap, fmt, ops::Deref, ops::DerefMut, sync::Arc};

use next_web_core::{async_trait, traits::required::Required};

use crate::{
    authentication::AuthenticationProvider,
    config::{
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{
            configurers::BaseHttpConfigurer,
            HttpSecurityBuilder,
        },
    },
    core::{
        Authentication, GrantedAuthority, Principal,
        {AuthenticationError, AuthenticationErrorKind},
    },
    core::simple_granted_authority::SimpleGrantedAuthority,
    oauth2_resource_server::bearer::BearerTokenAuthenticationToken,
    web::authentication::AuthPrincipal,
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

/// The principal returned from opaque token introspection.
#[derive(Clone)]
pub struct OAuth2AuthenticatedPrincipal {
    name: String,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
    attributes: HashMap<String, String>,
}

impl OAuth2AuthenticatedPrincipal {
    pub fn new(
        name: impl Into<String>,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
        attributes: HashMap<String, String>,
    ) -> Self {
        Self {
            name: name.into(),
            authorities,
            attributes,
        }
    }

    pub fn attributes(&self) -> &HashMap<String, String> {
        &self.attributes
    }
}

impl Principal for OAuth2AuthenticatedPrincipal {
    fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for OAuth2AuthenticatedPrincipal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OAuth2AuthenticatedPrincipal [name={}, authorities={}]", self.name, self.authorities.len())
    }
}

/// The metadata describing an OAuth2 protected resource (the
/// `/.well-known/oauth-protected-resource` document).
#[derive(Clone, Debug, Default)]
pub struct OAuth2ProtectedResourceMetadata {
    resource_id: Option<String>,
    authorization_servers: Vec<String>,
    resource_signing_algorithms: Vec<String>,
    jwk_set_url: Option<String>,
}

impl OAuth2ProtectedResourceMetadata {
    pub fn resource_id(&self) -> Option<&String> {
        self.resource_id.as_ref()
    }

    pub fn set_resource_id(&mut self, resource_id: impl Into<String>) {
        self.resource_id = Some(resource_id.into());
    }

    pub fn authorization_servers(&self) -> &[String] {
        &self.authorization_servers
    }

    pub fn set_authorization_servers(&mut self, authorization_servers: Vec<String>) {
        self.authorization_servers = authorization_servers;
    }

    pub fn jwk_set_url(&self) -> Option<&String> {
        self.jwk_set_url.as_ref()
    }

    pub fn set_jwk_set_url(&mut self, jwk_set_url: impl Into<String>) {
        self.jwk_set_url = Some(jwk_set_url.into());
    }
}

/// Introspects an opaque bearer token, returning the associated authenticated
/// principal.
pub trait OpaqueTokenIntrospector: Send + Sync {
    fn introspect(&self, token: &str) -> Result<OAuth2AuthenticatedPrincipal, AuthenticationError>;
}

/// The default introspector backed by Spring's `OAuth2AuthenticatedPrincipal`
/// introspection endpoint. Introspection is not implemented in this port.
#[derive(Clone, Debug)]
pub struct SpringOpaqueTokenIntrospector {
    introspection_uri: String,
    client_id: Option<String>,
    client_secret: Option<String>,
}

impl SpringOpaqueTokenIntrospector {
    pub fn new(
        introspection_uri: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> Self {
        Self {
            introspection_uri: introspection_uri.into(),
            client_id: Some(client_id.into()),
            client_secret: Some(client_secret.into()),
        }
    }
}

impl OpaqueTokenIntrospector for SpringOpaqueTokenIntrospector {
    fn introspect(&self, _token: &str) -> Result<OAuth2AuthenticatedPrincipal, AuthenticationError> {
        Err(AuthenticationError::with_kind(
            "Opaque token introspection is not implemented",
            AuthenticationErrorKind::BadCredentials,
        ))
    }
}

/// Converts an introspected principal into the authenticated bearer token
/// principal and authorities.
pub trait OpaqueTokenAuthenticationConverter: Send + Sync {
    fn convert(
        &self,
        bearer_token: &BearerTokenAuthenticationToken,
        principal: &OAuth2AuthenticatedPrincipal,
    ) -> (AuthPrincipal, Vec<Arc<dyn GrantedAuthority>>);
}

#[derive(Clone, Debug, Default)]
pub struct DefaultOpaqueTokenAuthenticationConverter;

impl OpaqueTokenAuthenticationConverter for DefaultOpaqueTokenAuthenticationConverter {
    fn convert(
        &self,
        _bearer_token: &BearerTokenAuthenticationToken,
        principal: &OAuth2AuthenticatedPrincipal,
    ) -> (AuthPrincipal, Vec<Arc<dyn GrantedAuthority>>) {
        (Arc::new(principal.clone()), principal.authorities.clone())
    }
}

/// The `AuthenticationProvider` for opaque bearer tokens.
#[derive(Clone)]
pub struct OpaqueTokenAuthenticationProvider {
    introspector: Arc<dyn OpaqueTokenIntrospector>,
    authentication_converter: Arc<dyn OpaqueTokenAuthenticationConverter>,
}

impl OpaqueTokenAuthenticationProvider {
    pub fn new(
        introspector: Arc<dyn OpaqueTokenIntrospector>,
        authentication_converter: Arc<dyn OpaqueTokenAuthenticationConverter>,
    ) -> Self {
        Self {
            introspector,
            authentication_converter,
        }
    }

    pub fn with_default_converter(introspector: Arc<dyn OpaqueTokenIntrospector>) -> Self {
        Self::new(introspector, Arc::new(DefaultOpaqueTokenAuthenticationConverter::default()))
    }
}

#[async_trait]
impl AuthenticationProvider for OpaqueTokenAuthenticationProvider {
    fn supports(&self, authentication: TypeId) -> bool {
        authentication == TypeId::of::<BearerTokenAuthenticationToken>()
    }

    async fn authenticate(
        &self,
        authentication: &Arc<dyn Authentication>,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        let as_any: &dyn Any = authentication.as_ref();
        let token = as_any
            .downcast_ref::<BearerTokenAuthenticationToken>()
            .ok_or_else(|| {
                AuthenticationError::with_kind(
                    "Unsupported authentication type",
                    AuthenticationErrorKind::ProviderNotFound,
                )
            })?;
        let bearer = token.token().ok_or_else(|| {
            AuthenticationError::with_kind(
                "Bearer token is missing",
                AuthenticationErrorKind::BadCredentials,
            )
        })?;
        let principal = self.introspector.introspect(bearer)?;
        let (principal, authorities) =
            self.authentication_converter.convert(token, &principal);
        Ok(Some(Arc::new(BearerTokenAuthenticationToken::authenticated(
            principal, authorities, bearer,
        ))))
    }
}

/// The HTTP configurer for opaque token resource server support.
#[derive(Clone)]
pub struct OpaqueTokenConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    introspector: Option<Arc<dyn OpaqueTokenIntrospector>>,
    authentication_converter: Arc<dyn OpaqueTokenAuthenticationConverter>,
    opaque_token_authentication_provider: Option<Arc<OpaqueTokenAuthenticationProvider>>,
    base: BaseHttpConfigurer<OpaqueTokenConfigurer<H>, H>,
}

impl<H> OpaqueTokenConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    pub fn new() -> Self {
        Self {
            introspector: None,
            authentication_converter: Arc::new(DefaultOpaqueTokenAuthenticationConverter::default()),
            opaque_token_authentication_provider: None,
            base: Default::default(),
        }
    }

    /// Sets the `OpaqueTokenIntrospector` to use.
    pub fn introspector(&mut self, introspector: Arc<dyn OpaqueTokenIntrospector>) -> &mut Self {
        self.introspector = Some(introspector);
        self
    }

    /// Configures a `SpringOpaqueTokenIntrospector` from an introspection URI and
    /// client credentials.
    pub fn introspection_uri(
        &mut self,
        introspection_uri: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> &mut Self {
        self.introspector = Some(Arc::new(SpringOpaqueTokenIntrospector::new(
            introspection_uri,
            client_id,
            client_secret,
        )));
        self
    }

    pub fn authentication_converter(
        &mut self,
        authentication_converter: Arc<dyn OpaqueTokenAuthenticationConverter>,
    ) -> &mut Self {
        self.authentication_converter = authentication_converter;
        self
    }

    pub fn get_introspector(&self) -> Option<&Arc<dyn OpaqueTokenIntrospector>> {
        self.introspector.as_ref()
    }

    pub fn authentication_provider(&mut self) -> Arc<OpaqueTokenAuthenticationProvider> {
        if let Some(provider) = self.opaque_token_authentication_provider.clone() {
            return provider;
        }
        let introspector = self.introspector.clone().unwrap_or_else(|| {
            Arc::new(SpringOpaqueTokenIntrospector::new("", "", ""))
        });
        let provider = Arc::new(OpaqueTokenAuthenticationProvider::new(
            introspector,
            self.authentication_converter.clone(),
        ));
        self.opaque_token_authentication_provider = Some(provider.clone());
        provider
    }
}

impl<H> Deref for OpaqueTokenConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<OpaqueTokenConfigurer<H>, H>;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.base
    }
}

impl<H> DerefMut for OpaqueTokenConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.base
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for OpaqueTokenConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: crate::config::security_builder::SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for OpaqueTokenConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: crate::config::security_builder::SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn init(&mut self, http: &mut H) {
        let provider = self.authentication_provider();
        http.authentication_provider(provider);
    }

    fn configure(&mut self, _http: &mut H) {}
}

impl<H> Default for OpaqueTokenConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self::new()
    }
}
