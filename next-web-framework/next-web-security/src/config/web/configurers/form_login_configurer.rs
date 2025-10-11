use std::marker::PhantomData;

use next_web_core::traits::Required::Required;

use crate::{
    config::{
        security_builder::SecurityBuilder, security_configurer::SecurityConfigurer, security_configurer_adapter::SecurityConfigurerAdapter, web::{
            configurers::{
                abstract_authentication_filter_configurer::{
                    AbstractAuthenticationFilterConfigurer, AuthenticationFilterConfigurer,
                },
                abstract_http_configurer::AbstractHttpConfigurer,
            },
            http_security_builder::HttpSecurityBuilder,
        }
    },
    web::{authentication::{
        forward_authentication_failure_handler::ForwardAuthenticationFailureHandler,
        forward_authentication_success_handler::ForwardAuthenticationSuccessHandler,
        ui::default_login_page_generating_filter::DefaultLoginPageGeneratingFilter,
        username_password_authentication_filter::UsernamePasswordAuthenticationFilter,
    }, default_security_filter_chain::DefaultSecurityFilterChain},
};

#[derive(Clone)]
pub struct FormLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: AuthenticationFilterConfigurer<H>,
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
    H: HttpSecurityBuilder<H>,
    H: AuthenticationFilterConfigurer<H>,
    H: Clone,
{
    pub fn username_parameter(&mut self, username_parameter: &str) -> &mut Self {
        self.abstract_authentication_filter_configurer
            .get_mut_authentication_filter()
            .set_username_parameter(username_parameter);
        self
    }

    pub fn password_parameter(&mut self, password_parameter: &str) -> &mut Self {
        self.abstract_authentication_filter_configurer
            .get_mut_authentication_filter()
            .set_password_parameter(password_parameter);
        self
    }

    pub fn failure_forward_url(&mut self, forward_url: &str) -> &mut Self {
        self.abstract_authentication_filter_configurer
            .failure_handler(ForwardAuthenticationFailureHandler::new(forward_url));
        self
    }

    pub fn success_forward_url(&mut self, forward_url: &str) -> &mut Self {
        self.abstract_authentication_filter_configurer
            .success_handler(ForwardAuthenticationSuccessHandler::new(forward_url));
        self
    }

    fn get_username_parameter(&self) -> &str {
        self.abstract_authentication_filter_configurer
            .get_authentication_filter()
            .get_username_parameter()
    }

    fn get_password_parameter(&self) -> &str {
        self.abstract_authentication_filter_configurer
            .get_authentication_filter()
            .get_password_parameter()
    }

    fn init_default_login_filter(&mut self, http: &mut H) {
        let login_page_generating_filter =
            http.get_mut_shared_object::<DefaultLoginPageGeneratingFilter>();
        let abs = &self.abstract_authentication_filter_configurer;
        if !self
            .abstract_authentication_filter_configurer
            .is_custom_login_page()
        {
            if let Some(login_page_generating_filter) = login_page_generating_filter {
                login_page_generating_filter.set_form_login_enabled(true);
                login_page_generating_filter.set_username_parameter(self.get_password_parameter());
                login_page_generating_filter.set_password_parameter(self.get_password_parameter());
                login_page_generating_filter.set_login_page_url(abs.get_login_page());
                login_page_generating_filter
                    .set_failure_url(abs.get_failure_url().unwrap_or_default());
                login_page_generating_filter
                    .set_authentication_url(abs.get_login_processing_url().unwrap_or_default());
            }
        }
    }
}
impl<H> Required<UsernamePasswordAuthenticationFilter> for FormLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: AuthenticationFilterConfigurer<H>,
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
    H: HttpSecurityBuilder<H>,
    H: AuthenticationFilterConfigurer<H>,
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
    H: HttpSecurityBuilder<H>,
    H: AuthenticationFilterConfigurer<H>,
    H: Clone,
{
    fn get_object(&self) -> &AbstractHttpConfigurer<FormLoginConfigurer<H>, H> {
        &self.abstract_http_configurer
    }

    fn get_object_mut(&mut self) -> &mut AbstractHttpConfigurer<FormLoginConfigurer<H>, H> {
        &mut self.abstract_http_configurer
    }
}

impl<H> AuthenticationFilterConfigurer<H> for FormLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: AuthenticationFilterConfigurer<H>,
    H: Clone,
{
    fn login_processing_url(&mut self, login_processing_url: &str) {
        self.abstract_authentication_filter_configurer
            .login_processing_url(login_processing_url);
    }

    fn login_page(&mut self, login_page: &str) {
        self.abstract_authentication_filter_configurer
            .login_page(login_page);
    }
}

impl<H> SecurityConfigurer<FormLoginConfigurer<H>, H> for FormLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: AuthenticationFilterConfigurer<H>,
    H: Clone,
{
    fn init(&mut self, http: &mut H) {
        self.abstract_authentication_filter_configurer.init(http);

        self.init_default_login_filter(http);
    }

    fn configure(&mut self, http: &mut H) {
        self.abstract_authentication_filter_configurer
            .configure(http);
    }
}

impl<H> Default for FormLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: AuthenticationFilterConfigurer<H>,
    H: Clone,
{
    fn default() -> Self {
        let authentication_filter = UsernamePasswordAuthenticationFilter::default();

        let abstract_authentication_filter_configurer =
            AbstractAuthenticationFilterConfigurer::new(authentication_filter, None);

        let abstract_http_configurer = AbstractHttpConfigurer::default();
        let mut form_login_configurer = Self {
            abstract_authentication_filter_configurer,
            abstract_http_configurer,
            username_password_authentication_filter: Default::default(),
            _marker: PhantomData,
        };

        form_login_configurer.username_parameter("username");
        form_login_configurer.password_parameter("password");

        form_login_configurer
    }
}


impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>> for FormLoginConfigurer<H>
where 
H:  SecurityBuilder<DefaultSecurityFilterChain>,
H:  HttpSecurityBuilder<H>,
H:  AuthenticationFilterConfigurer<H> + Clone
{
    fn get_object(&self) -> & SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        todo!()
    }

    fn get_object_mut(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        todo!()
    }
}