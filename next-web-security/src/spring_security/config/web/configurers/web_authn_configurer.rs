use std::ops::{Deref, DerefMut};

use crate::{
    config::{
        security_configurer::SecurityConfigurer,
        web::{
            builders::HttpSecurity, configurers::BaseHttpConfigurer,
            http_security_builder::HttpSecurityBuilder,
        },
    },
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

#[derive(Clone)]
pub struct WebAuthnConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    inner: BaseHttpConfigurer<Self, H>,
}

impl<H> Default for WebAuthnConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, HttpSecurity> for WebAuthnConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, builder: &mut HttpSecurity) {
        todo!()
    }

    fn configure(&mut self, builder: &mut HttpSecurity) {
        todo!()
    }
}

impl<H> Deref for WebAuthnConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<H> DerefMut for WebAuthnConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
