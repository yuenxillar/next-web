use crate::{
    config::{security_builder::SecurityBuilder, security_configurer::SecurityConfigurer},
    core::filter::Filter,
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

pub trait HttpSecurityBuilder<H>
where
    Self: Send + Sync,
    Self: SecurityBuilder<DefaultSecurityFilterChain>,
    H: HttpSecurityBuilder<H>,
{
    fn add_filter(&mut self, filter: impl Filter) -> H;

    fn get_configurer<T>(&self) -> Option<T>
    where
        T: SecurityConfigurer<DefaultSecurityFilterChain, H>;
}
