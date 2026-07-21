use std::ops::{Deref, DerefMut};

use crate::{
    config::{
        security_configurer::SecurityConfigurer,
        web::{configurers::BaseHttpConfigurer, http_security_builder::HttpSecurityBuilder},
    },
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

#[derive(Clone)]
pub struct OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    inner: BaseHttpConfigurer<Self, B>,
}

impl<B> Default for OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl<B> SecurityConfigurer<DefaultSecurityFilterChain, B> for OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    fn init(&mut self, builder: &mut B) {
        todo!()
    }

    fn configure(&mut self, builder: &mut B) {
        todo!()
    }
}

impl<B> Deref for OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    type Target = BaseHttpConfigurer<Self, B>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<B> DerefMut for OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
