use std::marker::PhantomData;

use crate::{config::web::{configurers::abstract_authentication_filter_configurer::AbstractAuthenticationFilterConfigurer, http_security_builder::HttpSecurityBuilder}, web::authentication::username_password_authentication_filter::UsernamePasswordAuthenticationFilter};

pub struct FormLoginConfigurer<H: HttpSecurityBuilder<H>> {
    _marker: PhantomData<H>,

    username_password_authentication_filter: UsernamePasswordAuthenticationFilter,

    abstract_authentication_filter_configurer: AbstractAuthenticationFilterConfigurer<H>,
}

impl<H: HttpSecurityBuilder<H>> FormLoginConfigurer<H> {}

impl<H: HttpSecurityBuilder<H>> Default for FormLoginConfigurer<H> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
            username_password_authentication_filter: Default::default(),
        }
    }
}
