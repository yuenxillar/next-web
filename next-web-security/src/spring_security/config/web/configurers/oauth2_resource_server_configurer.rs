use std::ops::{Deref, DerefMut};

use next_web_core::{traits::required::Required, ApplicationContext};

use crate::{
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{configurers::BaseHttpConfigurer, http_security_builder::HttpSecurityBuilder},
    },
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

/// Configures OAuth 2.0 Resource Server support.
///
/// When fully implemented, this configurer will:
/// - Create `BearerTokenAuthenticationFilter` to authenticate JWT or opaque bearer tokens
/// - Register `JwtAuthenticationProvider` (JWT mode) or `OpaqueTokenAuthenticationProvider` (opaque mode)
/// - Create `OAuth2ProtectedResourceMetadataFilter` for `/.well-known/oauth-protected-resource`
/// - Register `BearerTokenAuthenticationEntryPoint` and `BearerTokenAccessDeniedHandler`
///
/// Sub-configurers:
/// - `.jwt(customizer)` — JWT bearer token validation
/// - `.opaque_token(customizer)` — Opaque token introspection
///
/// Requires: `JwtDecoder` (JWT mode) or `OpaqueTokenIntrospector` (opaque mode).
#[derive(Clone)]
pub struct OAuth2ResourceServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    inner: BaseHttpConfigurer<OAuth2ResourceServerConfigurer<H>, H>,
}

impl<H> OAuth2ResourceServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    pub fn new(ctx: &ApplicationContext) -> Self {
        Self::default()
    }
}

impl<H> Default for OAuth2ResourceServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for OAuth2ResourceServerConfigurer<H>
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

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for OAuth2ResourceServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {
        // Stub: Requires JwtDecoder or OpaqueTokenIntrospector infrastructure.
        // When implemented:
        //   - Register JwtAuthenticationProvider or OpaqueTokenAuthenticationProvider
        //   - Register BearerTokenAuthenticationEntryPoint with ExceptionHandlingConfigurer
        //   - Register BearerTokenAccessDeniedHandler
        //   - Configure CSRF to ignore bearer token requests
    }

    fn configure(&mut self, _http: &mut H) {
        // Stub: Will create BearerTokenAuthenticationFilter when
        // JWT / opaque token infrastructure is ready.
    }
}

impl<H> Deref for OAuth2ResourceServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<H> DerefMut for OAuth2ResourceServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
