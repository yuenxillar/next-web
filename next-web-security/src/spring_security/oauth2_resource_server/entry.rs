use std::{fmt, ops::Deref, ops::DerefMut, sync::Arc};

use next_web_core::{
    async_trait, http::StatusCode, filter::FilterError, traits::required::Required,
    traits::filter::{HttpFilter, HttpFilterChain},
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
    traits::named::Named, error::BoxError,
};

use crate::{
    config::{
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{
            configurers::BaseHttpConfigurer,
            HttpSecurityBuilder,
        },
    },
    core::AuthenticationError,
    web::{
        access::AccessDeniedHandler,
        authentication::AuthenticationFailureHandler,
        default_security_filter_chain::DefaultSecurityFilterChain,
        AuthenticationEntryPoint,
    },
};

use super::opaque::OAuth2ProtectedResourceMetadata;

/// The default `AuthenticationEntryPoint` for bearer token resource servers.
/// Responds with `401 Unauthorized` and a `WWW-Authenticate: Bearer` header.
#[derive(Clone, Debug, Default)]
pub struct BearerTokenAuthenticationEntryPoint;

impl AuthenticationEntryPoint for BearerTokenAuthenticationEntryPoint {
    fn commence(
        &self,
        _request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        auth_error: &AuthenticationError,
    ) -> Result<(), BoxError> {
        response.set_status_code(StatusCode::UNAUTHORIZED);
        let error_description = auth_error.to_string();
        let header_value = format!(
            "Bearer error=\"invalid_token\", error_description=\"{error_description}\""
        );
        response.insert_header("WWW-Authenticate", header_value.as_str());
        response.set_body(Vec::new());
        Ok(())
    }
}

/// The default `AccessDeniedHandler` for bearer token resource servers. Responds
/// with `403 Forbidden` and a `WWW-Authenticate: Bearer` header carrying the
/// `insufficient_scope` error.
#[derive(Clone, Debug, Default)]
pub struct BearerTokenAccessDeniedHandler;

impl AccessDeniedHandler for BearerTokenAccessDeniedHandler {
    fn handle(
        &self,
        _request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _access_denied_error: &crate::access::AccessDeniedError,
    ) -> Result<(), BoxError> {
        response.set_status_code(StatusCode::FORBIDDEN);
        response.insert_header("WWW-Authenticate", "Bearer error=\"insufficient_scope\"");
        Ok(())
    }
}

/// An `AuthenticationFailureHandler` that delegates to a configured
/// `AuthenticationEntryPoint`, mirroring Spring's
/// `AuthenticationEntryPointFailureHandler`.
#[derive(Clone, Debug)]
pub struct AuthenticationEntryPointFailureHandler {
    entry_point: Arc<dyn AuthenticationEntryPoint>,
}

impl AuthenticationEntryPointFailureHandler {
    pub fn new(entry_point: Arc<dyn AuthenticationEntryPoint>) -> Self {
        Self { entry_point }
    }
}

impl fmt::Debug for dyn AuthenticationEntryPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "dyn AuthenticationEntryPoint")
    }
}

impl AuthenticationFailureHandler for AuthenticationEntryPointFailureHandler {
    fn on_authentication_failure(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        error: &AuthenticationError,
    ) -> Result<(), BoxError> {
        self.entry_point.commence(request, response, error)
    }
}

/// A filter that publishes the `/.well-known/oauth-protected-resource`
/// document describing this resource server.
#[derive(Clone)]
pub struct OAuth2ProtectedResourceMetadataFilter {
    metadata_uri: String,
    metadata: OAuth2ProtectedResourceMetadata,
}

impl OAuth2ProtectedResourceMetadataFilter {
    pub fn new(metadata: OAuth2ProtectedResourceMetadata) -> Self {
        Self {
            metadata_uri: "/.well-known/oauth-protected-resource".to_string(),
            metadata,
        }
    }

    pub fn metadata_uri(mut self, metadata_uri: impl Into<String>) -> Self {
        self.metadata_uri = metadata_uri.into();
        self
    }
}

#[async_trait]
impl HttpFilter for OAuth2ProtectedResourceMetadataFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if request.path() == self.metadata_uri {
            let mut servers = String::new();
            for (index, server) in self.metadata.authorization_servers().iter().enumerate() {
                if index > 0 {
                    servers.push_str(", ");
                }
                servers.push_str(&format!("\"{server}\""));
            }
            let jwk = self
                .metadata
                .jwk_set_url()
                .map(|url| format!(",\n  \"jwks_uri\": \"{url}\""))
                .unwrap_or_default();
            let body = format!(
                "{{\n  \"resource_id\": \"{}\",\n  \"authorization_servers\": [{}]{}\n}}",
                self.metadata
                    .resource_id()
                    .cloned()
                    .unwrap_or_default(),
                servers,
                jwk
            );
            response.set_status_code(StatusCode::OK);
            response.insert_header("Content-Type", "application/json");
            response.set_body(body.into_bytes());
            return Ok(());
        }
        filter_chain.do_filter(request, response).await
    }

    fn supports(&self, name: &str) -> bool {
        name == "oAuth2ProtectedResourceMetadataFilter"
    }
}

impl Named for OAuth2ProtectedResourceMetadataFilter {
    fn name(&self) -> &str {
        "oAuth2ProtectedResourceMetadataFilter"
    }
}

/// The HTTP configurer for OAuth2 DPoP (Demonstrating Proof-of-Possession)
/// resource server support. DPoP proof validation is not implemented in this
/// port; the configurer mirrors the Java DSL and delegates decoding to a
/// `JwtAuthenticationProvider` when a decoder is supplied.
#[derive(Clone)]
pub struct DPoPConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    jwt_decoder: Option<Arc<dyn super::jwt::JwtDecoder>>,
    authentication_converter: Arc<dyn super::jwt::JwtAuthenticationConverter>,
    base: BaseHttpConfigurer<DPoPConfigurer<H>, H>,
}

impl<H> DPoPConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    pub fn new() -> Self {
        Self {
            jwt_decoder: None,
            authentication_converter: Arc::new(
                super::jwt::DefaultJwtAuthenticationConverter::default(),
            ),
            base: Default::default(),
        }
    }

    pub fn decoder(&mut self, decoder: Arc<dyn super::jwt::JwtDecoder>) -> &mut Self {
        self.jwt_decoder = Some(decoder);
        self
    }

    pub fn jwt_authentication_converter(
        &mut self,
        jwt_authentication_converter: Arc<dyn super::jwt::JwtAuthenticationConverter>,
    ) -> &mut Self {
        self.authentication_converter = jwt_authentication_converter;
        self
    }

    pub fn get_decoder(&self) -> Option<&Arc<dyn super::jwt::JwtDecoder>> {
        self.jwt_decoder.as_ref()
    }
}

impl<H> Deref for DPoPConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<DPoPConfigurer<H>, H>;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.base
    }
}

impl<H> DerefMut for DPoPConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.base
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>> for DPoPConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: crate::config::security_builder::SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(
        &self,
    ) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_object()
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for DPoPConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: crate::config::security_builder::SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn init(&mut self, http: &mut H) {
        if let Some(decoder) = self.jwt_decoder.clone() {
            let provider = Arc::new(super::jwt::JwtAuthenticationProvider::new(
                decoder,
                self.authentication_converter.clone(),
            ));
            http.authentication_provider(provider);
        }
    }

    fn configure(&mut self, _http: &mut H) {}
}

impl<H> Default for DPoPConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self::new()
    }
}

/// The HTTP configurer that publishes the OAuth2 protected resource metadata
/// document.
#[derive(Clone)]
pub struct ProtectedResourceMetadataConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    metadata: OAuth2ProtectedResourceMetadata,
    base: BaseHttpConfigurer<ProtectedResourceMetadataConfigurer<H>, H>,
}

impl<H> ProtectedResourceMetadataConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    pub fn new() -> Self {
        Self {
            metadata: OAuth2ProtectedResourceMetadata::default(),
            base: Default::default(),
        }
    }

    pub fn resource_id(&mut self, resource_id: impl Into<String>) -> &mut Self {
        self.metadata.set_resource_id(resource_id);
        self
    }

    pub fn authorization_server(&mut self, authorization_server: impl Into<String>) -> &mut Self {
        self.metadata
            .set_authorization_servers(vec![authorization_server.into()]);
        self
    }

    pub fn jwk_set_uri(&mut self, jwk_set_uri: impl Into<String>) -> &mut Self {
        self.metadata.set_jwk_set_url(jwk_set_uri);
        self
    }

    pub fn metadata(&self) -> &OAuth2ProtectedResourceMetadata {
        &self.metadata
    }
}

impl<H> Deref for ProtectedResourceMetadataConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<ProtectedResourceMetadataConfigurer<H>, H>;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.base
    }
}

impl<H> DerefMut for ProtectedResourceMetadataConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.base
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for ProtectedResourceMetadataConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: crate::config::security_builder::SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(
        &self,
    ) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_object()
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H>
    for ProtectedResourceMetadataConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: crate::config::security_builder::SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn init(&mut self, _http: &mut H) {}

    fn configure(&mut self, http: &mut H) {
        http.add_filter(OAuth2ProtectedResourceMetadataFilter::new(self.metadata.clone()));
    }
}

impl<H> Default for ProtectedResourceMetadataConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self::new()
    }
}
