use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt,
    ops::Deref,
    ops::DerefMut,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use jsonwebtoken::{Algorithm, DecodingKey, Header, Validation};
use next_web_core::{async_trait, Required};
use serde_json::Value;

use crate::{
    authentication::AuthenticationProvider,
    core::authority::SimpleGrantedAuthority,
    core::{Authentication, AuthenticationError, GrantedAuthority},
    oauth2::jose::{Jwk, JwkSet, JwkSource},
    oauth2::server::resource::bearer::BearerTokenAuthenticationToken,
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
#[async_trait]
pub trait JwtDecoder: Send + Sync {
    /// Decodes a token, verifying its signature and its claims.
    ///
    /// # Arguments
    ///
    /// * `token` - The token to decode.
    async fn decode(&self, token: &str) -> Result<Jwt, AuthenticationError>;
}

/// Loads the keys of an authorization server from a JWK set URI.
///
/// A deployment that verifies the tokens of another service loads the JWK set
/// of that service with an HTTP client. The decoder asks the loader for the set
/// once and keeps it, so the loader may load the set on every call, or serve a
/// cached one.
#[async_trait]
pub trait JwkSetLoader: Send + Sync {
    /// Loads the JWK set of the given URI.
    ///
    /// # Arguments
    ///
    /// * `jwk_set_uri` - The URI of the JWK set.
    async fn load(&self, jwk_set_uri: &str) -> Result<JwkSet, AuthenticationError>;
}

/// Loads the JWK set of an authorization server over HTTP.
///
/// The loader is the default of [`NimbusJwtDecoder::from_jwk_set_uri`], so a
/// decoder that is configured with the URI of the JWK set of a server works
/// without any further wiring. The decoder keeps the set it read, and it asks
/// the loader for the set again when a token names a key the set does not hold.
#[derive(Clone, Debug, Default)]
pub struct RemoteJwkSetLoader {
    http_client: reqwest::Client,
}

impl RemoteJwkSetLoader {
    /// Creates a loader that reads the JWK set with its own HTTP client.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a loader that reads the JWK set with the given HTTP client.
    ///
    /// # Arguments
    ///
    /// * `http_client` - The HTTP client of the loader.
    pub fn with_http_client(http_client: reqwest::Client) -> Self {
        Self { http_client }
    }
}

#[async_trait]
impl JwkSetLoader for RemoteJwkSetLoader {
    async fn load(&self, jwk_set_uri: &str) -> Result<JwkSet, AuthenticationError> {
        let response = self
            .http_client
            .get(jwk_set_uri)
            .send()
            .await
            .map_err(|error| {
                invalid_token(format!(
                    "The JWK set of the authorization server cannot be read: {error}"
                ))
            })?;

        if !response.status().is_success() {
            return Err(invalid_token(format!(
                "The JWK set of the authorization server reports the status {}",
                response.status()
            )));
        }

        response.json::<JwkSet>().await.map_err(|error| {
            invalid_token(format!(
                "The JWK set of the authorization server is not a valid document: {error}"
            ))
        })
    }
}

/// A JWT decoder that verifies the signature of a token and its claims.
///
/// The decoder verifies the token with the keys of the authorization server
/// that issued it, and it refuses a token that it cannot verify: a token whose
/// key is unknown, whose signature does not match, whose algorithm is unknown,
/// or whose registered claims are not valid is rejected. The keys are read from
/// the JWK set that was contributed, or, when only a JWK set URI was
/// configured, they are loaded with the [`JwkSetLoader`] of the decoder and
/// kept for the tokens that follow, until the lifetime of the keys passes.
///
/// The keys of a server rotate, so the keys a decoder keeps are read again when
/// their lifetime passes, and again when a token names a key that the decoder
/// does not hold. A decoder that reads the keys of another service therefore
/// follows a rotation without a restart of the resource server.
#[derive(Clone)]
pub struct NimbusJwtDecoder {
    jwk_set_uri: Option<String>,
    jwk_set_loader: Option<Arc<dyn JwkSetLoader>>,
    jwk_source: Option<Arc<dyn JwkSource>>,
    jwk_set: Arc<RwLock<Option<CachedJwkSet>>>,
    jwk_set_time_to_live: Duration,
    issuer: Option<Vec<String>>,
    audience: Option<Vec<String>>,
    leeway: u64,
}

/// The keys a decoder keeps, and the time they are read again.
#[derive(Clone)]
struct CachedJwkSet {
    jwk_set: JwkSet,
    expires_at: Instant,
}

impl NimbusJwtDecoder {
    /// Creates a decoder that reads the keys of the given JWK set URI.
    ///
    /// The keys are loaded with [`NimbusJwtDecoder::set_jwk_set_loader`] or
    /// [`NimbusJwtDecoder::set_jwk_source`] before a token is decoded; a
    /// decoder that has no keys refuses every token.
    ///
    /// # Arguments
    ///
    /// * `jwk_set_uri` - The URI of the JWK set of the authorization server.
    pub fn from_jwk_set_uri(jwk_set_uri: impl Into<String>) -> Self {
        Self {
            jwk_set_uri: Some(jwk_set_uri.into()),
            jwk_set_loader: Some(Arc::new(RemoteJwkSetLoader::default())),
            ..Self::default()
        }
    }

    /// Sets the keys the tokens are verified with.
    ///
    /// # Arguments
    ///
    /// * `jwk_source` - The keys of the authorization server.
    pub fn set_jwk_source(&mut self, jwk_source: Arc<dyn JwkSource>) {
        self.jwk_source = Some(jwk_source);
    }

    /// Sets the keys the tokens are verified with, builder style.
    ///
    /// # Arguments
    ///
    /// * `jwk_source` - The keys of the authorization server.
    pub fn with_jwk_source(mut self, jwk_source: Arc<dyn JwkSource>) -> Self {
        self.set_jwk_source(jwk_source);
        self
    }

    /// Sets the loader that reads the JWK set URI of the decoder.
    ///
    /// # Arguments
    ///
    /// * `jwk_set_loader` - The loader of the JWK set.
    pub fn set_jwk_set_loader(&mut self, jwk_set_loader: Arc<dyn JwkSetLoader>) {
        self.jwk_set_loader = Some(jwk_set_loader);
    }

    /// Sets the loader that reads the JWK set URI of the decoder, builder style.
    ///
    /// # Arguments
    ///
    /// * `jwk_set_loader` - The loader of the JWK set.
    pub fn with_jwk_set_loader(mut self, jwk_set_loader: Arc<dyn JwkSetLoader>) -> Self {
        self.set_jwk_set_loader(jwk_set_loader);
        self
    }

    /// Sets the issuers a token has to name.
    ///
    /// # Arguments
    ///
    /// * `issuer` - The issuer, or the issuers, of the tokens.
    pub fn set_issuer<I, S>(&mut self, issuer: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.issuer = Some(issuer.into_iter().map(Into::into).collect());
    }

    /// Sets the audiences a token has to name.
    ///
    /// # Arguments
    ///
    /// * `audience` - The audience, or the audiences, of the tokens.
    pub fn set_audience<I, S>(&mut self, audience: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.audience = Some(audience.into_iter().map(Into::into).collect());
    }

    /// Sets the clock skew that is tolerated when the registered claims are
    /// verified.
    ///
    /// # Arguments
    ///
    /// * `leeway` - The tolerated skew, in seconds.
    pub fn set_leeway(&mut self, leeway: u64) {
        self.leeway = leeway;
    }

    /// Sets the lifetime of the keys the decoder keeps, in seconds.
    ///
    /// The keys of the JWK set are read again when the lifetime passes, which
    /// is what lets a resource server follow the rotation of the keys of the
    /// authorization server. The default lifetime is five minutes.
    ///
    /// # Arguments
    ///
    /// * `jwk_set_time_to_live` - The lifetime, in seconds.
    pub fn set_jwk_set_time_to_live(&mut self, jwk_set_time_to_live: u64) {
        self.jwk_set_time_to_live = Duration::from_secs(jwk_set_time_to_live);
    }

    /// Sets the lifetime of the keys the decoder keeps, builder style.
    ///
    /// # Arguments
    ///
    /// * `jwk_set_time_to_live` - The lifetime, in seconds.
    pub fn with_jwk_set_time_to_live(mut self, jwk_set_time_to_live: u64) -> Self {
        self.set_jwk_set_time_to_live(jwk_set_time_to_live);
        self
    }

    /// Returns the keys the tokens are verified with.
    async fn jwk_set(&self) -> Result<JwkSet, AuthenticationError> {
        if let Some(jwk_source) = self.jwk_source.as_ref() {
            return Ok(jwk_source.jwk_set());
        }

        // The keys are kept while their lifetime has not passed, so a decoder
        // that verifies many tokens reads the set of the server once.
        if let Some(cached) = self.cached_jwk_set() {
            if Instant::now() < cached.expires_at {
                return Ok(cached.jwk_set);
            }
        }

        let (Some(uri), Some(loader)) = (self.jwk_set_uri.as_ref(), self.jwk_set_loader.as_ref())
        else {
            // The decoder cannot read the set again, so the keys it has are the
            // keys it verifies with.
            if let Some(cached) = self.cached_jwk_set() {
                return Ok(cached.jwk_set);
            }
            return Err(invalid_token(
                "The decoder has no keys to verify the token with",
            ));
        };

        let jwk_set = loader.load(uri).await?;
        self.store_jwk_set(jwk_set.clone());

        Ok(jwk_set)
    }

    /// Reads the keys of the JWK set URI again, replacing the keys the decoder
    /// keeps.
    ///
    /// The decoder reads the keys again when a token names a key that its keys
    /// do not hold, which is what lets an authorization server rotate its keys
    /// without a restart of the resource server.
    async fn refresh_jwk_set(&self) -> Result<JwkSet, AuthenticationError> {
        let (Some(uri), Some(loader)) = (self.jwk_set_uri.as_ref(), self.jwk_set_loader.as_ref())
        else {
            return Err(invalid_token(
                "The decoder has no keys to verify the token with",
            ));
        };

        let jwk_set = loader.load(uri).await?;
        self.store_jwk_set(jwk_set.clone());

        Ok(jwk_set)
    }

    /// Returns the keys the decoder keeps, if it has any.
    fn cached_jwk_set(&self) -> Option<CachedJwkSet> {
        self.jwk_set
            .read()
            .expect("the JWK set lock is not poisoned")
            .clone()
    }

    /// Keeps the keys for the lifetime of the decoder.
    ///
    /// # Arguments
    ///
    /// * `jwk_set` - The keys of the authorization server.
    fn store_jwk_set(&self, jwk_set: JwkSet) {
        *self
            .jwk_set
            .write()
            .expect("the JWK set lock is not poisoned") = Some(CachedJwkSet {
            jwk_set,
            expires_at: Instant::now() + self.jwk_set_time_to_live,
        });
    }

    /// Returns the key that verifies the token.
    ///
    /// # Arguments
    ///
    /// * `header` - The header of the token.
    async fn decoding_key(&self, header: &Header) -> Result<DecodingKey, AuthenticationError> {
        let jwk_set = self.jwk_set().await?;
        if let Some(jwk) = key_of(&jwk_set, header) {
            return decoding_key_with(jwk, header);
        }

        // The token names a key the set does not hold. The server may have
        // rotated its keys since the set was read, so the set is read again once
        // before the token is refused.
        let refreshed = self.refresh_jwk_set().await?;
        let jwk = key_of(&refreshed, header)
            .ok_or_else(|| invalid_token("The key of the token is unknown"))?;

        decoding_key_with(jwk, header)
    }

    /// Returns the validation the decoder applies to a token.
    ///
    /// The registered claims of the specification are verified, the issuer and
    /// the audience are verified when they are configured, and the tolerated
    /// clock skew is applied.
    ///
    /// # Arguments
    ///
    /// * `algorithm` - The algorithm of the token.
    fn validation(&self, algorithm: Algorithm) -> Validation {
        let mut validation = Validation::new(algorithm);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.leeway = self.leeway;

        match self.issuer.as_ref() {
            Some(issuer) => validation.set_issuer(issuer),
            None => validation.validate_exp = true,
        }
        match self.audience.as_ref() {
            Some(audience) => validation.set_audience(audience),
            None => validation.validate_aud = false,
        }

        validation
    }
}

/// Returns the key of the set that verifies the token, if the set holds one.
///
/// # Arguments
///
/// * `jwk_set` - The keys of the authorization server.
/// * `header` - The header of the token.
fn key_of<'a>(jwk_set: &'a JwkSet, header: &Header) -> Option<&'a Jwk> {
    match header.kid.as_deref() {
        Some(kid) => jwk_set.find_by_kid(kid),
        None => jwk_set.first(),
    }
}

/// Builds the key that verifies a token with the key of the server.
///
/// # Arguments
///
/// * `jwk` - The key of the authorization server.
/// * `header` - The header of the token.
fn decoding_key_with(jwk: &Jwk, header: &Header) -> Result<DecodingKey, AuthenticationError> {
    if !algorithm_is_compatible(jwk, header.alg) {
        return Err(invalid_token(
            "The algorithm of the token does not match the key of the authorization server",
        ));
    }

    decoding_key_of(jwk)
}

impl Default for NimbusJwtDecoder {
    fn default() -> Self {
        Self {
            jwk_set_uri: None,
            jwk_set_loader: None,
            jwk_source: None,
            jwk_set: Arc::new(RwLock::new(None)),
            // The keys of a server are read again after five minutes, which is
            // the lifetime the keys of a server are kept by default.
            jwk_set_time_to_live: Duration::from_secs(300),
            issuer: None,
            audience: None,
            leeway: 0,
        }
    }
}

impl fmt::Debug for NimbusJwtDecoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NimbusJwtDecoder")
            .field("jwk_set_uri", &self.jwk_set_uri)
            .field("has_keys", &self.jwk_source.is_some())
            .field("jwk_set_time_to_live", &self.jwk_set_time_to_live)
            .field("issuer", &self.issuer)
            .field("audience", &self.audience)
            .field("leeway", &self.leeway)
            .finish()
    }
}

#[async_trait]
impl JwtDecoder for NimbusJwtDecoder {
    async fn decode(&self, token: &str) -> Result<Jwt, AuthenticationError> {
        let header = jsonwebtoken::decode_header(token)
            .map_err(|_| invalid_token("The token is not a well-formed JWT"))?;

        let decoding_key = self.decoding_key(&header).await?;
        let validation = self.validation(header.alg);

        let decoded = jsonwebtoken::decode::<Value>(token, &decoding_key, &validation)
            .map_err(|error| invalid_token(format!("The token is not valid: {error}")))?;

        let headers = flatten(serde_json::to_value(&decoded.header).unwrap_or(Value::Null));
        let claims = flatten(decoded.claims);

        Ok(Jwt::new(token).with_headers(headers).with_claims(claims))
    }
}

/// Returns whether the algorithm of a token can be verified with the key.
///
/// A symmetric key only verifies a token that is signed symmetrically, and an
/// RSA key only verifies a token that is signed with RSA, which is what keeps a
/// public key from being used as a shared secret.
///
/// # Arguments
///
/// * `jwk` - The key of the authorization server.
/// * `algorithm` - The algorithm of the token.
fn algorithm_is_compatible(jwk: &Jwk, algorithm: Algorithm) -> bool {
    match jwk.kty() {
        "oct" => matches!(
            algorithm,
            Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512
        ),
        "RSA" => matches!(
            algorithm,
            Algorithm::RS256
                | Algorithm::RS384
                | Algorithm::RS512
                | Algorithm::PS256
                | Algorithm::PS384
                | Algorithm::PS512
        ),
        _ => false,
    }
}

/// Builds the key that verifies a token from the key of the server.
///
/// # Arguments
///
/// * `jwk` - The key of the authorization server.
fn decoding_key_of(jwk: &Jwk) -> Result<DecodingKey, AuthenticationError> {
    match jwk.kty() {
        "oct" => jwk
            .secret()
            .map(|secret| DecodingKey::from_secret(&secret))
            .ok_or_else(|| {
                invalid_token("The symmetric key of the authorization server is invalid")
            }),
        "RSA" => {
            let (Some(modulus), Some(exponent)) = (jwk.modulus(), jwk.exponent()) else {
                return Err(invalid_token(
                    "The RSA key of the authorization server is incomplete",
                ));
            };
            DecodingKey::from_rsa_components(modulus, exponent)
                .map_err(|_| invalid_token("The RSA key of the authorization server is invalid"))
        }
        _ => Err(invalid_token(
            "The type of the key of the authorization server is not supported",
        )),
    }
}

/// Returns the error a token that cannot be verified reports.
///
/// # Arguments
///
/// * `message` - The reason the token is rejected.
fn invalid_token(message: impl Into<String>) -> AuthenticationError {
    AuthenticationError::with_kind(
        message,
        crate::core::AuthenticationErrorKind::BadCredentials,
    )
}

/// Flattens a JSON document into the string map the [`Jwt`] model uses.
///
/// # Arguments
///
/// * `value` - The JSON document.
fn flatten(value: Value) -> HashMap<String, String> {
    value
        .as_object()
        .map(|object| {
            object
                .iter()
                .map(|(name, value)| {
                    let value = match value {
                        Value::String(value) => value.clone(),
                        value => value.to_string(),
                    };
                    (name.clone(), value)
                })
                .collect()
        })
        .unwrap_or_default()
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
        Self::new(
            jwt_decoder,
            Arc::new(DefaultJwtAuthenticationConverter::default()),
        )
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
        let jwt = self.jwt_decoder.decode(bearer).await?;
        let (principal, authorities) = self.jwt_authentication_converter.convert(&jwt)?;
        Ok(Some(Arc::new(
            BearerTokenAuthenticationToken::authenticated(principal, authorities, bearer),
        )))
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
    base: crate::config::web::configurers::BaseHttpConfigurer<JwtConfigurer<H>, H>,
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
        let provider = Arc::new(JwtAuthenticationProvider::new(
            decoder,
            self.jwt_authentication_converter.clone(),
        ));
        self.jwt_authentication_provider = Some(provider.clone());
        provider
    }
}

impl<H> Deref for JwtConfigurer<H>
where
    H: crate::config::web::HttpSecurityBuilder<H>,
{
    type Target = crate::config::web::configurers::BaseHttpConfigurer<JwtConfigurer<H>, H>;

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

impl<H>
    Required<
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

impl<H>
    crate::config::security_configurer::SecurityConfigurer<
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

/// The tests sign the tokens they decode, and only a build that supports the
/// authorization server carries a signer.
#[cfg(all(test, feature = "oauth2-authorization-server"))]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use serde_json::json;

    use super::{JwkSetLoader, JwtDecoder, NimbusJwtDecoder};
    use crate::oauth2::jose::{
        base64_url, Jwk, JwkSet, JwkSource, JwtSigner, RsaJwtSigner, SecretJwtSigner,
    };

    /// A key source that serves a fixed set of keys.
    struct FixedJwkSet(JwkSet);

    impl JwkSource for FixedJwkSet {
        fn jwk_set(&self) -> JwkSet {
            self.0.clone()
        }
    }

    /// A loader that counts the sets it was asked for.
    struct CountingLoader(AtomicUsize, JwkSet);

    #[next_web_core::async_trait]
    impl JwkSetLoader for CountingLoader {
        async fn load(
            &self,
            _jwk_set_uri: &str,
        ) -> Result<JwkSet, crate::core::AuthenticationError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(self.1.clone())
        }
    }

    /// Returns claims that are valid far beyond the tests.
    fn claims(expires_at: u64) -> serde_json::Value {
        json!({
            "iss": "https://server.example",
            "aud": "demo-client",
            "sub": "user",
            "scope": "read",
            "iat": 1_700_000_000,
            "exp": expires_at,
        })
    }

    /// Returns a decoder that verifies the tokens with the given keys.
    fn decoder(jwk_source: Arc<dyn JwkSource>) -> NimbusJwtDecoder {
        NimbusJwtDecoder::default().with_jwk_source(jwk_source)
    }

    #[tokio::test]
    async fn a_symmetric_token_is_verified_with_the_secret_of_the_server() {
        let signer = SecretJwtSigner::new("secret");
        let token = signer
            .sign(&claims(4_102_444_800))
            .expect("the token is signed");
        let decoder = decoder(Arc::new(FixedJwkSet(signer.jwk_set())));

        let jwt = decoder.decode(&token).await.expect("the token is verified");

        assert_eq!(jwt.claim("sub").map(String::as_str), Some("user"));
        assert_eq!(jwt.claim("scope").map(String::as_str), Some("read"));
        assert_eq!(jwt.header("alg").map(String::as_str), Some("HS256"));
        assert_eq!(jwt.token_value(), token);
    }

    #[tokio::test]
    async fn an_rsa_token_is_verified_with_the_public_key_of_the_server() {
        let signer = RsaJwtSigner::generate(2048).expect("the key is generated");
        let token = signer
            .sign(&claims(4_102_444_800))
            .expect("the token is signed");
        let jwk_set = signer.jwk_set();
        let decoder = decoder(Arc::new(FixedJwkSet(jwk_set.clone())));

        let jwt = decoder.decode(&token).await.expect("the token is verified");

        assert_eq!(jwt.claim("sub").map(String::as_str), Some("user"));
        assert_eq!(jwt.header("alg").map(String::as_str), Some("RS256"));
        assert_eq!(
            jwt.header("kid").map(String::as_str),
            jwk_set.first().and_then(Jwk::kid)
        );
    }

    #[tokio::test]
    async fn a_token_signed_with_another_key_is_rejected() {
        let signer = SecretJwtSigner::new("secret");
        let token = signer
            .sign(&claims(4_102_444_800))
            .expect("the token is signed");
        let other = SecretJwtSigner::new("another-secret");
        let decoder = decoder(Arc::new(FixedJwkSet(other.jwk_set())));

        let error = decoder
            .decode(&token)
            .await
            .expect_err("the signature of the token does not match");

        assert_eq!(
            error.kind(),
            crate::core::AuthenticationErrorKind::BadCredentials
        );
    }

    #[tokio::test]
    async fn a_tampered_token_is_rejected() {
        let signer = SecretJwtSigner::new("secret");
        let token = signer
            .sign(&claims(4_102_444_800))
            .expect("the token is signed");
        let decoder = decoder(Arc::new(FixedJwkSet(signer.jwk_set())));

        // Keep the signature of the token and replace the claims with the claims
        // of another principal.
        let mut parts = token.split('.').collect::<Vec<_>>();
        let payload = base64_url(
            json!({
                "iss": "https://server.example",
                "aud": "demo-client",
                "sub": "admin",
                "exp": 4_102_444_800_u64,
            })
            .to_string()
            .as_bytes(),
        );
        parts[1] = &payload;
        let tampered = parts.join(".");

        assert!(
            decoder.decode(&tampered).await.is_err(),
            "the token is rejected"
        );
    }

    #[tokio::test]
    async fn an_unsigned_token_is_rejected() {
        // The classic attack: a token that claims not to be signed, which no
        // decoder may accept.
        let header = base64_url(b"{\"alg\":\"none\",\"typ\":\"JWT\"}");
        let payload = base64_url(
            json!({ "sub": "admin", "exp": 4_102_444_800_u64 })
                .to_string()
                .as_bytes(),
        );
        let token = format!("{header}.{payload}.");
        let signer = SecretJwtSigner::new("secret");
        let decoder = decoder(Arc::new(FixedJwkSet(signer.jwk_set())));

        assert!(
            decoder.decode(&token).await.is_err(),
            "the token is rejected"
        );
    }

    #[tokio::test]
    async fn a_token_with_a_key_of_another_type_is_rejected() {
        // The algorithm confusion attack: a token that is signed with RSA, but
        // whose identifier names a symmetric key.
        let signer = RsaJwtSigner::generate(2048).expect("the key is generated");
        let token = signer
            .sign(&claims(4_102_444_800))
            .expect("the token is signed");
        let kid = signer.public_jwk().kid().expect("the key has an id");
        let mut symmetric = Jwk::symmetric("secret");
        symmetric.set_kid(kid);
        let decoder = decoder(Arc::new(FixedJwkSet(JwkSet::new(vec![symmetric]))));

        assert!(
            decoder.decode(&token).await.is_err(),
            "the token is rejected"
        );
    }

    #[tokio::test]
    async fn an_expired_token_is_rejected() {
        let signer = SecretJwtSigner::new("secret");
        let token = signer
            .sign(&claims(1_700_000_100))
            .expect("the token is signed");
        let decoder = decoder(Arc::new(FixedJwkSet(signer.jwk_set())));

        let error = decoder
            .decode(&token)
            .await
            .expect_err("the token is expired");

        assert_eq!(
            error.kind(),
            crate::core::AuthenticationErrorKind::BadCredentials
        );
    }

    #[tokio::test]
    async fn a_decoder_without_keys_rejects_every_token() {
        let signer = SecretJwtSigner::new("secret");
        let token = signer
            .sign(&claims(4_102_444_800))
            .expect("the token is signed");
        let decoder = NimbusJwtDecoder::default();

        let error = decoder
            .decode(&token)
            .await
            .expect_err("a decoder that has no keys refuses the token");

        assert_eq!(
            error.kind(),
            crate::core::AuthenticationErrorKind::BadCredentials
        );
    }

    #[tokio::test]
    async fn the_issuer_of_a_token_is_verified_when_it_is_configured() {
        let signer = SecretJwtSigner::new("secret");
        let token = signer
            .sign(&claims(4_102_444_800))
            .expect("the token is signed");
        let mut decoder = decoder(Arc::new(FixedJwkSet(signer.jwk_set())));
        decoder.set_issuer(["https://other.example"]);

        assert!(
            decoder.decode(&token).await.is_err(),
            "the issuer does not match"
        );

        decoder.set_issuer(["https://server.example"]);
        assert!(decoder.decode(&token).await.is_ok(), "the issuer matches");
    }

    #[tokio::test]
    async fn the_audience_of_a_token_is_verified_when_it_is_configured() {
        let signer = SecretJwtSigner::new("secret");
        let token = signer
            .sign(&claims(4_102_444_800))
            .expect("the token is signed");
        let mut decoder = decoder(Arc::new(FixedJwkSet(signer.jwk_set())));
        decoder.set_audience(["other-client"]);

        assert!(
            decoder.decode(&token).await.is_err(),
            "the audience does not match"
        );

        decoder.set_audience(["demo-client"]);
        assert!(decoder.decode(&token).await.is_ok(), "the audience matches");
    }

    #[tokio::test]
    async fn the_keys_of_a_remote_jwk_set_are_loaded_once() {
        let signer = SecretJwtSigner::new("secret");
        let token = signer
            .sign(&claims(4_102_444_800))
            .expect("the token is signed");
        let loader = Arc::new(CountingLoader(AtomicUsize::new(0), signer.jwk_set()));
        let decoder = NimbusJwtDecoder::from_jwk_set_uri("https://server.example/oauth2/jwks")
            .with_jwk_set_loader(loader.clone());

        assert!(decoder.decode(&token).await.is_ok());
        assert!(decoder.decode(&token).await.is_ok());

        assert_eq!(
            loader.0.load(Ordering::SeqCst),
            1,
            "the JWK set is loaded once"
        );
    }

    #[tokio::test]
    async fn the_keys_of_a_remote_jwk_set_are_loaded_again_when_they_expire() {
        let signer = SecretJwtSigner::new("secret");
        let token = signer
            .sign(&claims(4_102_444_800))
            .expect("the token is signed");
        let loader = Arc::new(CountingLoader(AtomicUsize::new(0), signer.jwk_set()));
        let decoder = NimbusJwtDecoder::from_jwk_set_uri("https://server.example/oauth2/jwks")
            // The keys expire as soon as they are kept, so every token reads
            // them again the way a token does after the lifetime passed.
            .with_jwk_set_time_to_live(0)
            .with_jwk_set_loader(loader.clone());

        assert!(decoder.decode(&token).await.is_ok());
        assert!(decoder.decode(&token).await.is_ok());

        assert_eq!(
            loader.0.load(Ordering::SeqCst),
            2,
            "the keys are read again after their lifetime passed"
        );
    }
}
