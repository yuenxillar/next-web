use next_web_core::traits::required::Required;

use crate::{
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{
            configurers::base_http_configurer::BaseHttpConfigurer,
            http_security_builder::HttpSecurityBuilder,
        },
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
    Self: Required<BaseHttpConfigurer<OAuth2LoginConfigurer<H>, H>>,
{
    base_http_configurer: BaseHttpConfigurer<OAuth2LoginConfigurer<H>, H>,
}

impl<H> Default for OAuth2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<OAuth2LoginConfigurer<H>, H>>,
{
    fn default() -> Self {
        Self {
            base_http_configurer: Default::default(),
        }
    }
}

impl<H> Required<BaseHttpConfigurer<OAuth2LoginConfigurer<H>, H>>
    for OAuth2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<OAuth2LoginConfigurer<H>, H> {
        &self.base_http_configurer
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut BaseHttpConfigurer<OAuth2LoginConfigurer<H>, H> {
        &mut self.base_http_configurer
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for OAuth2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base_http_configurer.get_object()
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base_http_configurer.get_mut_object()
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
