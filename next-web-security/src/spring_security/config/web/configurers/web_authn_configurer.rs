use next_web_core::traits::required::Required;

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
    base: BaseHttpConfigurer<Self, H>,
}

impl<H> Default for WebAuthnConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            base: Default::default(),
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

impl<H> Required<BaseHttpConfigurer<Self, H>> for WebAuthnConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<Self, H> {
        &self.base
    }

    fn get_mut_object(&mut self) -> &mut BaseHttpConfigurer<Self, H> {
        &mut self.base
    }
}
