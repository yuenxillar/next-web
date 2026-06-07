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

/// Configures SAML 2.0 Single Logout (SLO).
///
/// When fully implemented, this will:
/// - Create `Saml2LogoutRequestFilter` to handle incoming LogoutRequests from the IdP
/// - Create `Saml2LogoutResponseFilter` to handle incoming LogoutResponses from the IdP
/// - Create `Saml2RelyingPartyInitiatedLogoutFilter` for SP-initiated logout
/// - Inherit logout handlers from `LogoutConfigurer`
///
/// Requires: `RelyingPartyRegistrationRepository`.
#[derive(Clone)]
pub struct Saml2LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<Saml2LogoutConfigurer<H>, H>>,
{
    logout_url: Option<String>,

    base_http_configurer: BaseHttpConfigurer<Saml2LogoutConfigurer<H>, H>,
}

impl<H> Saml2LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<Saml2LogoutConfigurer<H>, H>>,
{
    /// Set the RP-initiated logout URL.
    /// Default: `"/logout/saml2/slo"`.
    pub fn logout_url(mut self, url: &str) -> Self {
        self.logout_url = Some(url.to_string());
        self
    }
}

impl<H> Default for Saml2LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<Saml2LogoutConfigurer<H>, H>>,
{
    fn default() -> Self {
        Self {
            logout_url: None,
            base_http_configurer: Default::default(),
        }
    }
}

impl<H> Required<BaseHttpConfigurer<Saml2LogoutConfigurer<H>, H>>
    for Saml2LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<Saml2LogoutConfigurer<H>, H> {
        &self.base_http_configurer
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut BaseHttpConfigurer<Saml2LogoutConfigurer<H>, H> {
        &mut self.base_http_configurer
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for Saml2LogoutConfigurer<H>
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

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for Saml2LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {
        // Stub: Requires SAML2 logout filters + RelyingPartyRegistrationRepository.
    }

    fn configure(&mut self, _http: &mut H) {
        // Stub: Will create Saml2LogoutRequestFilter, Saml2LogoutResponseFilter,
        // and Saml2RelyingPartyInitiatedLogoutFilter when SAML2 infra is ready.
    }
}
