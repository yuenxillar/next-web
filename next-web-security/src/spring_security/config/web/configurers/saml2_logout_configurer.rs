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
{
    logout_url: Option<String>,

    base: BaseHttpConfigurer<Saml2LogoutConfigurer<H>, H>,
}

impl<H> Saml2LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    pub fn new(ctx: &ApplicationContext) -> Self {
        Self::default()
    }

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
{
    fn default() -> Self {
        Self {
            logout_url: None,
            base: Default::default(),
        }
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for Saml2LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_mut_object()
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

impl<H> Deref for Saml2LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<H> DerefMut for Saml2LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
