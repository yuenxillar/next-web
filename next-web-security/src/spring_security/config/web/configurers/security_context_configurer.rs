use next_web_core::traits::required::Required;

use crate::{
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        web::{
            builders::HttpSecurity, configurers::BaseHttpConfigurer,
            http_security_builder::HttpSecurityBuilder,
        },
    },
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

#[derive(Clone)]
pub struct SecurityContextConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    base: BaseHttpConfigurer<Self, H>,
}

impl<H> Default for SecurityContextConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            base: Default::default(),
        }
    }
}

impl<H> Required<BaseHttpConfigurer<SecurityContextConfigurer<H>, H>>
    for SecurityContextConfigurer<H>
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

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, HttpSecurity>
    for SecurityContextConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, builer: &mut HttpSecurity) {
        todo!()
    }

    fn configure(&mut self, builer: &mut HttpSecurity) {
        todo!()
    }
}
