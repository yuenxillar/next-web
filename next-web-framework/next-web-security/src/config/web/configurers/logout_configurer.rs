use std::marker::PhantomData;

use next_web_core::traits::required::Required;

use crate::{
    config::{
        security_configurer::SecurityConfigurer,
        web::{
            configurers::abstract_http_configurer::AbstractHttpConfigurer,
            http_security_builder::HttpSecurityBuilder,
        },
    },
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

#[derive(Clone)]
pub struct LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<AbstractHttpConfigurer<LogoutConfigurer<H>, H>>,
{
    _marker: PhantomData<H>,
}

impl<H> LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<AbstractHttpConfigurer<LogoutConfigurer<H>, H>>,
{
    pub fn is_custom_logout_success(&self) -> bool {
        false
    }

    pub fn logout_success_url(&mut self, logout_success_url: &str) {}
}

impl<H> Required<AbstractHttpConfigurer<LogoutConfigurer<H>, H>> for LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &AbstractHttpConfigurer<LogoutConfigurer<H>, H> {
        todo!()
    }

    fn get_object_mut(&mut self) -> &mut AbstractHttpConfigurer<LogoutConfigurer<H>, H> {
        todo!()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, builer: &mut H) {
    }

    fn configure(&mut self, builer:&mut  H) {
        todo!()
    }
}
