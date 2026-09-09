use std::{
    collections::BTreeSet,
    ops::{Deref, DerefMut},
};

use crate::{
    config::{
        security_configurer::SecurityConfigurer,
        web::{configurers::BaseHttpConfigurer, http_security_builder::HttpSecurityBuilder},
    },
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

#[derive(Clone)]
pub struct WebAuthnConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    base: BaseHttpConfigurer<Self, H>,
    rp_id: Option<String>,
    rp_name: Option<String>,
    allowed_origins: BTreeSet<String>,
    disable_default_registration_page: bool,
}

impl<H> WebAuthnConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// The Relying Party id.
    pub fn rp_id(&mut self, value: impl Into<String>) -> &mut Self {
        self.rp_id = Some(value.into());
        self
    }

    /// Sets the relying party name
    pub fn rp_name(&mut self, value: impl Into<String>) -> &mut Self {
        self.rp_name = Some(value.into());
        self
    }

    /// Convenience method for allowedOrigins(Set)
    pub fn allowed_origins<I, S>(&mut self, origins: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_origins = origins.into_iter().map(Into::into).collect();
        self
    }

    /// Configures whether the default webauthn registration should be disabled. Setting it to true will
    /// prevent the configurer from registering the DefaultWebAuthnRegistrationPageGeneratingFilter.
    pub fn disable_default_registration_page(&mut self, disable: bool) -> &mut Self {
        self.disable_default_registration_page = disable;
        self
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for WebAuthnConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _builder: &mut H) {
        if let Some(id) = &self.rp_id {
            assert!(!id.trim().is_empty(), "rp_id cannot be empty");
        }
    }

    fn configure(&mut self, builder: &mut H) {
        if !self.disable_default_registration_page {
            builder.add_filter(crate::web::authentication::ui::DefaultResourcesFilter::webauthn());
        }
    }
}

impl<H> Deref for WebAuthnConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<H> DerefMut for WebAuthnConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<H> Default for WebAuthnConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            base: Default::default(),
            rp_id: None,
            rp_name: None,
            allowed_origins: BTreeSet::new(),
            disable_default_registration_page: false,
        }
    }
}
