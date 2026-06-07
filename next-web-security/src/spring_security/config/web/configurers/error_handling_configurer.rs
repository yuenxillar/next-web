use std::sync::Arc;

use next_web_core::traits::required::Required;

use crate::{
    config::{
        security_configurer::SecurityConfigurer,
        web::{
            configurers::base_http_configurer::BaseHttpConfigurer,
            http_security_builder::HttpSecurityBuilder,
        },
    },
    web::{access::AccessDeniedHandler, default_security_filter_chain::DefaultSecurityFilterChain},
};

pub struct ErrorHandlingConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    base_http_configurer: BaseHttpConfigurer<Self, H>,
}

impl<H> ErrorHandlingConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    pub fn get_access_denied_handler(&self) -> Option<Arc<dyn AccessDeniedHandler>> {
        todo!()
    }
}

impl<H> Required<BaseHttpConfigurer<ErrorHandlingConfigurer<H>, H>> for ErrorHandlingConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<ErrorHandlingConfigurer<H>, H> {
        &self.base_http_configurer
    }

    fn get_mut_object(&mut self) -> &mut BaseHttpConfigurer<ErrorHandlingConfigurer<H>, H> {
        &mut self.base_http_configurer
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for ErrorHandlingConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, builer: &mut H) {
        todo!()
    }

    fn configure(&mut self, builer: &mut H) {
        todo!()
    }
}
