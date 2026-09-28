use crate::{
    config::{
        security_configurer::SecurityConfigurer,
        web::{configurers::BaseHttpConfigurer, HttpSecurityBuilder},
    },
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

#[derive(Clone)]
pub struct DefaultLoginPageConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    base: BaseHttpConfigurer<Self, H>,
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for DefaultLoginPageConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, builder: &mut H) {}

    fn configure(&mut self, builder: &mut H) {}
}

impl<H> Default for DefaultLoginPageConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            base: Default::default(),
        }
    }
}
