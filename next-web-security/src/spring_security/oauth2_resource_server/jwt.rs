use std::{any::{Any, TypeId}, collections::HashMap, ops::Deref, ops::DerefMut, sync::Arc};

use next_web_core::{async_trait, traits::required::Required};

use crate::{
    authentication::AuthenticationProvider,
    core::{Authentication, GrantedAuthority, Principal, AuthenticationError},
    core::simple_granted_authority::SimpleGrantedAuthority,
    oauth2_resource_server::bearer::BearerTokenAuthenticationToken,
    web::authentication::AuthPrincipal,
};

/// A decoded JWT. The claims and headers are stored as string maps to keep the
/// port simple; richer typed access mirrors the Java `Jwt` model where needed.
#[derive(Clone, Debug)]
pub struct Jwt {
    token_value: String,
    headers: HashMap<String, String>,
    claims: HashMap<String, String>,
}

impl Jwt {
    pub fn new(token_value: impl Into<String>) -> Self {
        Self {
            token_value: token_value.into(),
            headers: HashMap::new(),
            claims: HashMap::new(),
        }
    }

    pub fn token_value(&self) -> &str {
        &self.token_value
    }

    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    pub fn claims(&self) -> &HashMap<String, String> {
        &self.claims
    }

    pub fn header(&self, name: &str) -> Option<&String> {
        self.headers.get(name)
    }

    pub fn claim(&self, name: &str) -> Option<&String> {
        self.claims.get(name)
    }

    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    pub fn with_claims(mut self, claims: HashMap<String, String>) -> Self {
        self.claims = claims;
        self
    }
}

/// Decodes and verifies a JWT.
pub trait JwtDecoder: Send + Sync {
    fn decode(&self, token: &str) -> Result<Jwt, AuthenticationError>;
}

/// The default `JwtDecoder` backed by Nimbus. Signature verification and JWK-set
/// resolution are not implemented in this port; decode returns
/// `AuthenticationErrorKind::InvalidBearerToken`.
#[derive(Clone, Debug)]
pub struct NimbusJwtDecoder {
    jwk_set_uri: Option<String>,
}

impl NimbusJwtDecoder {
    pub fn from_jwk_set_uri(jwk_set_uri: impl Into<String>) -> Self {
        Self {
            jwk_set_uri: Some(jwk_set_uri.into()),
        }
    }
}

impl Default for NimbusJwtDecoder {
    fn default() -> Self {
        Self { jwk_set_uri: None }
    }
}

impl JwtDecoder for NimbusJwtDecoder {
    fn decode(&self, _token: &str) -> Result<Jwt, AuthenticationError> {
        Err(AuthenticationError::with_kind(
            "NimbusJwtDecoder signature verification is not implemented",
            crate::core::AuthenticationErrorKind::BadCredentials,
        ))
    }
}

/// Converts a `Jwt` into the principal and authorities used to build the
/// authenticated `BearerTokenAuthenticationToken`.
pub trait JwtAuthenticationConverter: Send + Sync {
    fn convert(
        &self,
        jwt: &Jwt,
    ) -> Result<(AuthPrincipal, Vec<Arc<dyn GrantedAuthority>>), AuthenticationError>;
}

/// The default converter that uses the `sub` claim as the principal name and the
/// `scope`/`scp` claim (space separated) as authorities.
#[derive(Clone, Debug, Default)]
pub struct DefaultJwtAuthenticationConverter;

impl JwtAuthenticationConverter for DefaultJwtAuthenticationConverter {
    fn convert(
        &self,
        jwt: &Jwt,
    ) -> Result<(AuthPrincipal, Vec<Arc<dyn GrantedAuthority>>), AuthenticationError> {
        let principal_name = jwt
            .claim("sub")
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        let scope = jwt
            .claim("scope")
            .or_else(|| jwt.claim("scp"))
            .cloned()
            .unwrap_or_default();
        let authorities = scope
            .split_whitespace()
            .filter(|scope| !scope.is_empty())
            .map(|scope| {
                let authority = if scope.starts_with("SCOPE_") {
                    scope.to_string()
                } else {
                    format!("SCOPE_{scope}")
                };
                Arc::new(SimpleGrantedAuthority::new(authority)) as Arc<dyn GrantedAuthority>
            })
            .collect();
        Ok((Arc::new(principal_name), authorities))
    }
}

/// The `AuthenticationProvider` for JWT bearer tokens.
#[derive(Clone)]
pub struct JwtAuthenticationProvider {
    jwt_decoder: Arc<dyn JwtDecoder>,
    jwt_authentication_converter: Arc<dyn JwtAuthenticationConverter>,
}

impl JwtAuthenticationProvider {
    pub fn new(
        jwt_decoder: Arc<dyn JwtDecoder>,
        jwt_authentication_converter: Arc<dyn JwtAuthenticationConverter>,
    ) -> Self {
        Self {
            jwt_decoder,
            jwt_authentication_converter,
        }
    }

    pub fn with_default_converter(jwt_decoder: Arc<dyn JwtDecoder>) -> Self {
        Self::new(jwt_decoder, Arc::new(DefaultJwtAuthenticationConverter::default()))
    }
}

#[async_trait]
impl AuthenticationProvider for JwtAuthenticationProvider {
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
                    crate::core::AuthenticationErrorKind::ProviderNotFound,
                )
            })?;
        let bearer = token.token().ok_or_else(|| {
            AuthenticationError::with_kind(
                "Bearer token is missing",
                crate::core::AuthenticationErrorKind::BadCredentials,
            )
        })?;
        let jwt = self.jwt_decoder.decode(bearer)?;
        let (principal, authorities) = self.jwt_authentication_converter.convert(&jwt)?;
        Ok(Some(Arc::new(BearerTokenAuthenticationToken::authenticated(
            principal, authorities, bearer,
        ))))
    }
}

/// The HTTP configurer for JWT resource server support.
#[derive(Clone)]
pub struct JwtConfigurer<H>
where
    H: crate::config::web::HttpSecurityBuilder<H>,
{
    jwt_decoder: Option<Arc<dyn JwtDecoder>>,
    jwt_authentication_converter: Arc<dyn JwtAuthenticationConverter>,
    jwt_authentication_provider: Option<Arc<JwtAuthenticationProvider>>,
    base: crate::config::web::configurers::BaseHttpConfigurer<
        JwtConfigurer<H>,
        H,
    >,
}

impl<H> JwtConfigurer<H>
where
    H: crate::config::web::HttpSecurityBuilder<H>,
{
    pub fn new() -> Self {
        Self {
            jwt_decoder: None,
            jwt_authentication_converter: Arc::new(DefaultJwtAuthenticationConverter::default()),
            jwt_authentication_provider: None,
            base: Default::default(),
        }
    }

    /// Sets the `JwtDecoder` to use.
    pub fn decoder(&mut self, decoder: Arc<dyn JwtDecoder>) -> &mut Self {
        self.jwt_decoder = Some(decoder);
        self
    }

    /// Configures a `NimbusJwtDecoder` from a JWK set URI.
    pub fn jwk_set_uri(&mut self, jwk_set_uri: impl Into<String>) -> &mut Self {
        self.jwt_decoder = Some(Arc::new(NimbusJwtDecoder::from_jwk_set_uri(jwk_set_uri)));
        self
    }

    /// Sets the `JwtAuthenticationConverter` to use.
    pub fn jwt_authentication_converter(
        &mut self,
        jwt_authentication_converter: Arc<dyn JwtAuthenticationConverter>,
    ) -> &mut Self {
        self.jwt_authentication_converter = jwt_authentication_converter;
        self
    }

    pub fn get_decoder(&self) -> Option<&Arc<dyn JwtDecoder>> {
        self.jwt_decoder.as_ref()
    }

    pub fn authentication_provider(&mut self) -> Arc<JwtAuthenticationProvider> {
        if let Some(provider) = self.jwt_authentication_provider.clone() {
            return provider;
        }
        let decoder = self
            .jwt_decoder
            .clone()
            .unwrap_or_else(|| Arc::new(NimbusJwtDecoder::default()));
        let provider =
            Arc::new(JwtAuthenticationProvider::new(decoder, self.jwt_authentication_converter.clone()));
        self.jwt_authentication_provider = Some(provider.clone());
        provider
    }
}

impl<H> Deref for JwtConfigurer<H>
where
    H: crate::config::web::HttpSecurityBuilder<H>,
{
    type Target =
        crate::config::web::configurers::BaseHttpConfigurer<JwtConfigurer<H>, H>;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.base
    }
}

impl<H> DerefMut for JwtConfigurer<H>
where
    H: crate::config::web::HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.base
    }
}

impl<H> Required<
    crate::config::security_configurer_adapter::SecurityConfigurerAdapter<
        crate::web::default_security_filter_chain::DefaultSecurityFilterChain,
        H,
    >,
> for JwtConfigurer<H>
where
    H: crate::config::web::HttpSecurityBuilder<H>,
    H: crate::config::security_builder::SecurityBuilder<
        crate::web::default_security_filter_chain::DefaultSecurityFilterChain,
    >,
{
    fn get_object(
        &self,
    ) -> &crate::config::security_configurer_adapter::SecurityConfigurerAdapter<
        crate::web::default_security_filter_chain::DefaultSecurityFilterChain,
        H,
    > {
        self.base.get_object()
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut crate::config::security_configurer_adapter::SecurityConfigurerAdapter<
        crate::web::default_security_filter_chain::DefaultSecurityFilterChain,
        H,
    > {
        self.base.get_mut_object()
    }
}

impl<H> crate::config::security_configurer::SecurityConfigurer<
    crate::web::default_security_filter_chain::DefaultSecurityFilterChain,
    H,
> for JwtConfigurer<H>
where
    H: crate::config::web::HttpSecurityBuilder<H>,
    H: crate::config::security_builder::SecurityBuilder<
        crate::web::default_security_filter_chain::DefaultSecurityFilterChain,
    >,
{
    fn init(&mut self, http: &mut H) {
        let provider = self.authentication_provider();
        http.authentication_provider(provider);
    }

    fn configure(&mut self, _http: &mut H) {}
}

impl<H> Default for JwtConfigurer<H>
where
    H: crate::config::web::HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self::new()
    }
}
