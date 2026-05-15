use next_web_core::{traits::required::Required, ApplicationContext};

use crate::{
    config::{
        security_configurer_adapter::SecurityConfigurerAdapter, web::http_security::HttpSecurity,
    },
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

#[derive(Clone, Default)]
pub struct CsrfConfigurer {
    security_configurer_adapter: SecurityConfigurerAdapter<DefaultSecurityFilterChain, HttpSecurity>,
}

impl CsrfConfigurer {
    pub fn new(_ctx: &ApplicationContext) -> Self {
        Self::default()
    }
}

impl Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, HttpSecurity>>
    for CsrfConfigurer
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, HttpSecurity> {
        &self.security_configurer_adapter
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, HttpSecurity> {
        &mut self.security_configurer_adapter
    }
}
