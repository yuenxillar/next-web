use std::{
    any::{Any, TypeId},
    borrow::Cow,
    collections::HashMap,
    fmt,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{async_trait, traits::required::Required};

use crate::{
    authentication::AuthenticationProvider,
    config::{
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{configurers::BaseHttpConfigurer, HttpSecurityBuilder},
    },
    core::{
        Authentication, GrantedAuthority, Principal, {AuthenticationError, AuthenticationErrorKind},
    },
    oauth2::server::resource::bearer::BearerTokenAuthenticationToken,
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

    /// Returns the authorities of the principal.
    pub fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        &self.authorities
    }
}

impl Principal for OAuth2AuthenticatedPrincipal {
    fn name(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.name)
    }
}

impl fmt::Display for OAuth2AuthenticatedPrincipal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OAuth2AuthenticatedPrincipal [name={}, authorities={}]",
            self.name,
            self.authorities.len()
        )
    }
}
impl crate::web::authentication::Identity for OAuth2AuthenticatedPrincipal {
    fn as_any(&self) -> &dyn std::any::Any {
        self
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

    /// Returns the signing algorithms of the resource.
    pub fn resource_signing_algorithms(&self) -> &[String] {
        &self.resource_signing_algorithms
    }

    /// Sets the signing algorithms of the resource.
    ///
    /// # Arguments
    ///
    /// * `resource_signing_algorithms` - The algorithms the resource accepts.
    pub fn set_resource_signing_algorithms(&mut self, resource_signing_algorithms: Vec<String>) {
        self.resource_signing_algorithms = resource_signing_algorithms;
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
#[async_trait]
pub trait OpaqueTokenIntrospector: Send + Sync {
    /// Returns the principal of an opaque token.
    ///
    /// A token that is not active is reported as a bad credential, which is
    /// what ends the authentication of the request.
    ///
    /// # Arguments
    ///
    /// * `token` - The opaque token of the request.
    async fn introspect(&self, token: &str)
        -> Result<OAuth2AuthenticatedPrincipal, AuthenticationError>;
}

/// The introspector of an opaque token, backed by the introspection endpoint of
/// an authorization server.
///
/// The introspector asks the endpoint for the state of the token, which is what
/// the specification names token introspection. The endpoint is called with the
/// credentials of the resource server, and a token the endpoint reports as
/// inactive is rejected.
#[derive(Clone, Debug)]
pub struct SpringOpaqueTokenIntrospector {
    introspection_uri: String,
    client_id: String,
    client_secret: String,
    http_client: reqwest::Client,
}

impl SpringOpaqueTokenIntrospector {
    /// Creates an introspector that calls the given introspection endpoint with
    /// the given client credentials.
    ///
    /// # Arguments
    ///
    /// * `introspection_uri` - The URI of the introspection endpoint.
    /// * `client_id` - The identifier of the resource server.
    /// * `client_secret` - The secret of the resource server.
    pub fn new(
        introspection_uri: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> Self {
        Self {
            introspection_uri: introspection_uri.into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            http_client: reqwest::Client::new(),
        }
    }

    /// Creates an introspector that calls the endpoint with the given HTTP
    /// client.
    ///
    /// # Arguments
    ///
    /// * `introspection_uri` - The URI of the introspection endpoint.
    /// * `client_id` - The identifier of the resource server.
    /// * `client_secret` - The secret of the resource server.
    /// * `http_client` - The HTTP client of the introspector.
    pub fn with_http_client(
        introspection_uri: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        http_client: reqwest::Client,
    ) -> Self {
        Self {
            introspection_uri: introspection_uri.into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            http_client,
        }
    }

    /// Returns the URI of the introspection endpoint.
    pub fn introspection_uri(&self) -> &str {
        &self.introspection_uri
    }
}

#[async_trait]
impl OpaqueTokenIntrospector for SpringOpaqueTokenIntrospector {
    async fn introspect(
        &self,
        token: &str,
    ) -> Result<OAuth2AuthenticatedPrincipal, AuthenticationError> {
        if self.introspection_uri.trim().is_empty() {
            return Err(AuthenticationError::with_kind(
                "The introspection endpoint of the resource server is not configured",
                AuthenticationErrorKind::BadCredentials,
            ));
        }

        let mut request = self
            .http_client
            .post(&self.introspection_uri)
            .form(&[("token", token), ("token_type_hint", "access_token")]);
        if !self.client_id.is_empty() {
            request = request.basic_auth(&self.client_id, Some(&self.client_secret));
        }

        let response = request.send().await.map_err(|error| {
            AuthenticationError::with_kind(
                format!("The introspection endpoint cannot be reached: {error}"),
                AuthenticationErrorKind::BadCredentials,
            )
        })?;

        if !response.status().is_success() {
            return Err(AuthenticationError::with_kind(
                format!(
                    "The introspection endpoint reports the status {}",
                    response.status()
                ),
                AuthenticationErrorKind::BadCredentials,
            ));
        }

        let introspection = response.json::<serde_json::Value>().await.map_err(|error| {
            AuthenticationError::with_kind(
                format!("The introspection response is not a valid document: {error}"),
                AuthenticationErrorKind::BadCredentials,
            )
        })?;

        principal_of(&introspection)
    }
}

/// Builds the principal of an introspection response.
///
/// A response that does not report the token as active is rejected. The name of
/// the principal is the subject of the token, or the client the token was
/// issued to, and the scopes of the token become the authorities of the
/// principal.
///
/// # Arguments
///
/// * `introspection` - The document the introspection endpoint returned.
fn principal_of(
    introspection: &serde_json::Value,
) -> Result<OAuth2AuthenticatedPrincipal, AuthenticationError> {
    if !introspection
        .get("active")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return Err(AuthenticationError::with_kind(
            "The opaque token is not active",
            AuthenticationErrorKind::BadCredentials,
        ));
    }

    let name = introspection
        .get("sub")
        .or_else(|| introspection.get("client_id"))
        .or_else(|| introspection.get("username"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("anonymous")
        .to_string();

    let authorities = scopes_of(introspection)
        .into_iter()
        .map(|scope| {
            let authority = if scope.starts_with("SCOPE_") {
                scope
            } else {
                format!("SCOPE_{scope}")
            };
            Arc::new(crate::core::authority::SimpleGrantedAuthority::new(authority))
                as Arc<dyn GrantedAuthority>
        })
        .collect();

    Ok(OAuth2AuthenticatedPrincipal::new(
        name,
        authorities,
        attributes_of(introspection),
    ))
}

/// Returns the scopes an introspection response reports.
///
/// # Arguments
///
/// * `introspection` - The document the introspection endpoint returned.
fn scopes_of(introspection: &serde_json::Value) -> Vec<String> {
    match introspection.get("scope") {
        Some(serde_json::Value::String(scope)) => scope
            .split_whitespace()
            .filter(|scope| !scope.is_empty())
            .map(ToString::to_string)
            .collect(),
        Some(serde_json::Value::Array(scopes)) => scopes
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(ToString::to_string)
            .collect(),
        _ => introspection
            .get("scp")
            .and_then(serde_json::Value::as_array)
            .map(|scopes| {
                scopes
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// Returns the claims an introspection response reports as attributes.
///
/// # Arguments
///
/// * `introspection` - The document the introspection endpoint returned.
fn attributes_of(introspection: &serde_json::Value) -> HashMap<String, String> {
    introspection
        .as_object()
        .map(|claims| {
            claims
                .iter()
                .map(|(name, value)| {
                    let value = match value {
                        serde_json::Value::String(value) => value.clone(),
                        value => value.to_string(),
                    };
                    (name.clone(), value)
                })
                .collect()
        })
        .unwrap_or_default()
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
        Self::new(
            introspector,
            Arc::new(DefaultOpaqueTokenAuthenticationConverter::default()),
        )
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
        let principal = self.introspector.introspect(bearer).await?;
        let (principal, authorities) = self.authentication_converter.convert(token, &principal);
        Ok(Some(Arc::new(
            BearerTokenAuthenticationToken::authenticated(principal, authorities, bearer),
        )))
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
        let introspector = self
            .introspector
            .clone()
            .unwrap_or_else(|| Arc::new(SpringOpaqueTokenIntrospector::new("", "", "")));
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

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use axum::{
        extract::{Form, State},
        http::HeaderMap,
        routing::post,
        Json, Router,
    };
    use serde_json::json;

    use crate::core::Principal;

    use super::{
        OAuth2AuthenticatedPrincipal, OpaqueTokenIntrospector, SpringOpaqueTokenIntrospector,
    };

    /// The request the introspector made, kept so a test can assert on it.
    #[derive(Clone, Default)]
    struct Captured {
        authorization: Option<String>,
        token: Option<String>,
    }

    /// Serves an introspection endpoint that reports the token as active.
    async fn introspection_server(
        State(captured): State<Arc<Mutex<Captured>>>,
        headers: HeaderMap,
        Form(form): Form<HashMap<String, String>>,
    ) -> Json<serde_json::Value> {
        let mut captured = captured.lock().expect("the captured request is not poisoned");
        captured.authorization = headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .map(ToString::to_string);
        captured.token = form.get("token").cloned();
        drop(captured);

        Json(json!({
            "active": true,
            "sub": "user",
            "client_id": "demo-client",
            "scope": "read write",
            "exp": 4_102_444_800_u64,
        }))
    }

    async fn serve(captured: Arc<Mutex<Captured>>) -> String {
        let app = Router::new()
            .route("/oauth2/introspect", post(introspection_server))
            .with_state(captured);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("the introspection server listens");
        let address = listener.local_addr().expect("the listener has an address");

        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        format!("http://{address}/oauth2/introspect")
    }

    #[test]
    fn an_active_introspection_becomes_a_principal() {
        let introspection = json!({
            "active": true,
            "sub": "user",
            "scope": "read write",
            "client_id": "demo-client",
        });

        let principal =
            super::principal_of(&introspection).expect("the token is active");

        assert_eq!(principal.name().as_ref(), "user");
        assert_eq!(principal.authorities().len(), 2);
        assert_eq!(
            principal.attributes().get("client_id"),
            Some(&"demo-client".to_string())
        );
    }

    #[test]
    fn an_inactive_introspection_is_rejected() {
        let introspection = json!({ "active": false });

        assert!(super::principal_of(&introspection).is_err());
    }

    #[tokio::test]
    async fn the_introspector_calls_the_endpoint_with_the_credentials_of_the_server() {
        let captured = Arc::new(Mutex::new(Captured::default()));
        let uri = serve(captured.clone()).await;
        let introspector =
            SpringOpaqueTokenIntrospector::new(uri, "resource-server", "resource-secret");

        let principal = introspector
            .introspect("the-token")
            .await
            .expect("the token is active");

        assert_eq!(principal.name().as_ref(), "user");
        let captured = captured.lock().expect("the captured request is not poisoned");
        assert_eq!(
            captured.authorization.as_deref(),
            Some("Basic cmVzb3VyY2Utc2VydmVyOnJlc291cmNlLXNlY3JldA=="),
            "the client authenticates with the Authorization header"
        );
        assert_eq!(captured.token.as_deref(), Some("the-token"));
    }

    #[tokio::test]
    async fn the_introspector_without_an_endpoint_is_rejected() {
        let introspector = SpringOpaqueTokenIntrospector::new("", "", "");

        assert!(introspector.introspect("the-token").await.is_err());
    }

    /// Keeps the unused import of the principal type checked by the compiler.
    #[allow(dead_code)]
    fn principal_type_is_used(principal: &OAuth2AuthenticatedPrincipal) -> String {
        principal.name().to_string()
    }
}
