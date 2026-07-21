use std::ops::{Deref, DerefMut};

use next_web_core::traits::required::Required;

use crate::{
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{configurers::BaseHttpConfigurer, http_security_builder::HttpSecurityBuilder},
    },
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

/// Configures OAuth 2.1 Authorization Server support.
///
/// When fully implemented, this configurer will manage 10+ sub-configurers:
/// - `OAuth2ClientAuthenticationConfigurer` — client authentication
/// - `OAuth2AuthorizationEndpointConfigurer` — authorization endpoint
/// - `OAuth2TokenEndpointConfigurer` — token endpoint
/// - `OAuth2TokenIntrospectionEndpointConfigurer` — token introspection
/// - `OAuth2TokenRevocationEndpointConfigurer` — token revocation
/// - `OAuth2DeviceAuthorizationEndpointConfigurer` — device authorization
/// - `OAuth2DeviceVerificationEndpointConfigurer` — device verification
/// - `OAuth2PushedAuthorizationRequestEndpointConfigurer` — PAR
/// - `OAuth2ClientRegistrationEndpointConfigurer` — client registration
/// - `OidcConfigurer` — OpenID Connect 1.0 support
///
/// Key filters created:
/// - `AuthorizationServerContextFilter`
/// - `NimbusJwkSetEndpointFilter` (conditional)
/// - Multiple endpoint-specific filters
///
/// Requires: `RegisteredClientRepository`, `OAuth2AuthorizationService`,
/// `AuthorizationServerSettings`, `OAuth2TokenGenerator`, etc.
#[derive(Clone)]
pub struct OAuth2AuthorizationServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    inner: BaseHttpConfigurer<OAuth2AuthorizationServerConfigurer<H>, H>,
}

impl<H> Default for OAuth2AuthorizationServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl<H> Required<BaseHttpConfigurer<OAuth2AuthorizationServerConfigurer<H>, H>>
    for OAuth2AuthorizationServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<OAuth2AuthorizationServerConfigurer<H>, H> {
        &self.inner
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut BaseHttpConfigurer<OAuth2AuthorizationServerConfigurer<H>, H> {
        &mut self.inner
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for OAuth2AuthorizationServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.inner.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.inner.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for OAuth2AuthorizationServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {
        // Stub: Requires full OAuth2 authorization server infrastructure.
        // When implemented:
        //   - Initialize all sub-configurers
        //   - Validate issuer URL and authorization server settings
        //   - Configure CSRF and entry points for token endpoints
        //   - Conditionally enable OIDC (SessionRegistry, OidcConfigurer)
    }

    fn configure(&mut self, _http: &mut H) {
        // Stub: Will create AuthorizationServerContextFilter,
        // NimbusJwkSetEndpointFilter, and all endpoint filters
        // when OAuth2 infrastructure is ready.
    }
}

impl<H> Deref for OAuth2AuthorizationServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<H> DerefMut for OAuth2AuthorizationServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
