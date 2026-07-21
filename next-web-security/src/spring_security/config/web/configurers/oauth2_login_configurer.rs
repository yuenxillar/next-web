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

/// Configures OAuth 2.0 / OpenID Connect 1.0 Login authentication.
///
/// When fully implemented, this configurer will:
/// - Create `OAuth2AuthorizationRequestRedirectFilter` to redirect users to the provider
/// - Create `OAuth2LoginAuthenticationFilter` to handle the authorization code callback
/// - Register `OAuth2LoginAuthenticationProvider` and optionally `OidcAuthorizationCodeAuthenticationProvider`
/// - Integrate with `DefaultLoginPageGeneratingFilter` for auto-generated login links
///
/// Requires: `ClientRegistrationRepository` and `OAuth2AuthorizedClientRepository`.
#[derive(Clone)]
pub struct OAuth2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    inner: BaseHttpConfigurer<OAuth2LoginConfigurer<H>, H>,
}

impl<H> Default for OAuth2LoginConfigurer<H>
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
    for OAuth2LoginConfigurer<H>
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

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for OAuth2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {
        // Stub: Requires OAuth2 client infrastructure (ClientRegistrationRepository, etc.)
    }

    fn configure(&mut self, _http: &mut H) {
        // Stub: Will create OAuth2AuthorizationRequestRedirectFilter +
        // OAuth2LoginAuthenticationFilter when OAuth2 infrastructure is ready.
    }
}

impl<H> Deref for OAuth2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<H> DerefMut for OAuth2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
