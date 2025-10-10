use std::marker::PhantomData;

use next_web_core::traits::Required::Required;

use crate::{
    config::{
        security_builder::SecurityBuilder,
        web::{
            configurers::{
                abstract_authentication_filter_configurer::{
                    AbstractAuthenticationFilterConfigurer, AuthenticationFilterConfigurer,
                },
                abstract_http_configurer::AbstractHttpConfigurer,
            },
            http_security_builder::HttpSecurityBuilder,
        },
    },
    web::authentication::username_password_authentication_filter::UsernamePasswordAuthenticationFilter,
};

#[derive(Clone)]
pub struct FormLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H> + SecurityBuilder<H>,
    H: AuthenticationFilterConfigurer<H>,
    H: Required<UsernamePasswordAuthenticationFilter>,
    H: Clone,
{
    _marker: PhantomData<H>,

    username_password_authentication_filter: UsernamePasswordAuthenticationFilter,
    abstract_authentication_filter_configurer: AbstractAuthenticationFilterConfigurer<
        H,
        FormLoginConfigurer<H>,
        UsernamePasswordAuthenticationFilter,
    >,

    abstract_http_configurer: AbstractHttpConfigurer<FormLoginConfigurer<H>, H>,
}


impl<H> FormLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H> + SecurityBuilder<H>,
    H: AuthenticationFilterConfigurer<H>,
    H: Required<UsernamePasswordAuthenticationFilter>,
    H: Clone,
{

    pub fn username_parameter(&mut self, username_parameter: &str) {
        self.abstract_authentication_filter_configurer.get_authentication_filter().set
    }

}
impl<H> Required<UsernamePasswordAuthenticationFilter> for FormLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H> + SecurityBuilder<H>,
    H: AuthenticationFilterConfigurer<H>,
    H: Required<UsernamePasswordAuthenticationFilter>,
    H: Clone,
{
    fn get_object(&self) -> &UsernamePasswordAuthenticationFilter {
        &self.username_password_authentication_filter
    }

    fn get_object_mut(&mut self) -> &mut UsernamePasswordAuthenticationFilter {
        &mut self.username_password_authentication_filter
    }
}

impl<H>
    Required<
        AbstractAuthenticationFilterConfigurer<
            H,
            FormLoginConfigurer<H>,
            UsernamePasswordAuthenticationFilter,
        >,
    > for FormLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H> + SecurityBuilder<H>,
    H: AuthenticationFilterConfigurer<H>,
    H: Required<UsernamePasswordAuthenticationFilter>,
    H: Clone,
{
    fn get_object(
        &self,
    ) -> &AbstractAuthenticationFilterConfigurer<
        H,
        FormLoginConfigurer<H>,
        UsernamePasswordAuthenticationFilter,
    > {
        &self.abstract_authentication_filter_configurer
    }

    fn get_object_mut(
        &mut self,
    ) -> &mut AbstractAuthenticationFilterConfigurer<
        H,
        FormLoginConfigurer<H>,
        UsernamePasswordAuthenticationFilter,
    > {
        &mut self.abstract_authentication_filter_configurer
    }
}

impl<H> Required<AbstractHttpConfigurer<FormLoginConfigurer<H>, H>> for FormLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H> + SecurityBuilder<H>,
    H: AuthenticationFilterConfigurer<H>,
    H: Required<UsernamePasswordAuthenticationFilter>,
    H: Clone,
{
    fn get_object(&self) -> &AbstractHttpConfigurer<FormLoginConfigurer<H>, H> {
        &self.abstract_http_configurer
    }

    fn get_object_mut(&mut self) -> &mut AbstractHttpConfigurer<FormLoginConfigurer<H>, H> {
        &mut self.abstract_http_configurer
    }
}

impl<H> Default for FormLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H> + SecurityBuilder<H>,
    H: AuthenticationFilterConfigurer<H>,
    H: Required<UsernamePasswordAuthenticationFilter>,
    H: Clone,
{
    fn default() -> Self {
        let authentication_filter =
            UsernamePasswordAuthenticationFilter::default();

        let abstract_authentication_filter_configurer = AbstractAuthenticationFilterConfigurer::new(authentication_filter, None);
        
        
        let form_login_configurer = Self {
            _marker: PhantomData,
            username_password_authentication_filter: Default::default(),
            abstract_authentication_filter_configurer,
            abstract_http_configurer: todo!(),
        };

        form_login_configurer

        form_login_configurer
    }
}
