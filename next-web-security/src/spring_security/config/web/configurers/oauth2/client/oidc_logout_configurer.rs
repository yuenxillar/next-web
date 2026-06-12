use next_web_core::traits::required::Required;

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
    base: BaseHttpConfigurer<Self, B>,
}

impl<B> Default for OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    fn default() -> Self {
        Self {
            base: Default::default(),
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

impl<B> Required<BaseHttpConfigurer<Self, B>> for OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<Self, B> {
        &self.base
    }

    fn get_mut_object(&mut self) -> &mut BaseHttpConfigurer<Self, B> {
        &mut self.base
    }
}
