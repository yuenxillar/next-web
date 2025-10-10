use std::{marker::PhantomData, sync::Arc};

use next_web_core::{traits::Required::Required, util::http_method::HttpMethod};

use crate::{
    authorization::authentication_details_source::AuthenticationDetailsSource,
    config::{security_builder::SecurityBuilder, web::{
        configurers::{abstract_http_configurer::AbstractHttpConfigurer, logout_configurer::LogoutConfigurer},
        http_security_builder::HttpSecurityBuilder, util::matcher::{ant_path_request_matcher::AntPathRequestMatcher, request_matcher::RequestMatcher},
    }},
    web::{
        authentication::{
            abstract_authentication_processing_filter::AbstractAuthenticationProcessingFilter,
            authentication_failure_handler::AuthenticationFailureHandler,
            authentication_success_handler::AuthenticationSuccessHandler,
            login_url_authentication_entry_point::LoginUrlAuthenticationEntryPoint,
            saved_request_aware_authentication_success_handler::SavedRequestAwareAuthenticationSuccessHandler,
            simple_url_authentication_failure_handler::SimpleUrlAuthenticationFailureHandler,
        },
        authentication_entry_point::AuthenticationEntryPoint,
    },
};

#[derive(Clone)]
pub struct AbstractAuthenticationFilterConfigurer<B, T, F>
where
    B: HttpSecurityBuilder<B>,
    B: SecurityBuilder<B>,
    T: Required<AbstractAuthenticationFilterConfigurer<B, T, F>>,
    T: Required<AbstractHttpConfigurer<T, B>>,
    F: Required<AbstractAuthenticationProcessingFilter>,
{
    auth_filter: F,

    authentication_details_source: Option<Arc<dyn AuthenticationDetailsSource>>,
    default_success_handler: Arc<dyn AuthenticationSuccessHandler>,
    success_handler: Arc<dyn AuthenticationSuccessHandler>,

    authentication_entry_point: Option<LoginUrlAuthenticationEntryPoint>,
    custom_login_page: bool,
    login_page: Box<str>,
    login_processing_url: Option<Box<str>>,

    failure_handler: Option<Arc<dyn AuthenticationFailureHandler>>,
    permit_all: bool,
    failure_url: Option<Box<str>>,

    abstract_http_configurer: AbstractHttpConfigurer<T, B>,

    _marker_1: PhantomData<T>,
    _marker_2: PhantomData<B>,
}

impl<B, T, F> AbstractAuthenticationFilterConfigurer<B, T, F>
where
    B: HttpSecurityBuilder<B>,
    B: SecurityBuilder<B>,
    B: Clone,
    T: Required<AbstractAuthenticationFilterConfigurer<B, T, F>>,
    T: Required<AbstractHttpConfigurer<T, B>>,
    F: Required<AbstractAuthenticationProcessingFilter>,
{
    pub fn new(authentication_filter: F, default_login_processing_url: Option<Box<str>>) -> Self {
        let default_success_handler = Arc::new(SavedRequestAwareAuthenticationSuccessHandler::new());
        let mut configurer = Self {
            default_success_handler: default_success_handler.clone(),
            auth_filter: authentication_filter,
            success_handler: default_success_handler,

            abstract_http_configurer: AbstractHttpConfigurer::new(),
            authentication_details_source: Default::default(),
            authentication_entry_point: Default::default(),
            custom_login_page: Default::default(),
            login_page: Default::default(),
            login_processing_url: Default::default(),
            failure_handler: Default::default(),
            permit_all: Default::default(),
            failure_url: Default::default(),
          
            _marker_1: PhantomData,
            _marker_2: PhantomData,
        };
        configurer.set_login_page("/login");
        if let Some(url) = default_login_processing_url {
            configurer.login_processing_url(url.as_ref());
        }

        configurer
    }

    fn default_success_url(&mut self, default_success_url: &str, always_use: bool) {
        let mut handler = SavedRequestAwareAuthenticationSuccessHandler::new();
        handler.set_default_target_url(default_success_url);
        handler.set_always_use_default_target_url(always_use);
        self.default_success_handler = Arc::new(handler);
        self.success_handler = self.default_success_handler.clone();
    }

    pub fn authentication_details_source(
        &mut self,
        authentication_details_source: Arc<dyn AuthenticationDetailsSource>,
    ) {
        self.authentication_details_source = Some(authentication_details_source);
    }

    pub fn success_handler(&mut self, success_handler: Arc<dyn AuthenticationSuccessHandler>) {
        self.success_handler = success_handler;
    }

    pub const fn permit_all(&mut self) {
        self.permit_all = true;
    }

    pub fn failure_url(&mut self, authentication_failure_url: &str) {
        self.failure_handler(Arc::new(SimpleUrlAuthenticationFailureHandler::new(
            authentication_failure_url,
        )));
        self.failure_url = Some(authentication_failure_url.into());
    }

    pub fn failure_handler(
        &mut self,
        authentication_failure_handler: Arc<dyn AuthenticationFailureHandler>,
    ) {
        self.failure_url = None;
        self.failure_handler = Some(authentication_failure_handler);
    }

    pub fn is_custom_login_page(&self) -> bool {
        self.custom_login_page
    }

    pub fn get_authentication_filter(&self) -> &F {
        &self.auth_filter
    }

    pub fn set_authentication_filter(&mut self, auth_filter: F) {
        self.auth_filter = auth_filter;
    }

    pub fn get_login_page(&self) -> &str {
        self.login_page.as_ref()
    }

    pub fn get_authentication_entry_point(&self) -> Option<&dyn AuthenticationEntryPoint> {
        self.authentication_entry_point.as_ref().map(|ep| ep as &dyn AuthenticationEntryPoint)
    }

    pub fn get_login_processing_url(&self) -> Option<&str> {
        self.login_processing_url.as_deref()
    }

    pub fn get_failure_url(&self) -> Option<&str> {
        self.failure_url.as_deref()
    }
    
    pub fn update_authentication_defaults(&mut self) {
        let url = self.login_page.to_string();
        if self.login_processing_url.is_none() {
            self.login_processing_url(&url);
        }

        if self.failure_handler.is_none() {
            self.failure_url(&format!("{}?error", url));
        }


        let abstract_http_configurer = self.get_object();
        let security_configurer_adapter = abstract_http_configurer.get_object();

        let logout_configurer = match security_configurer_adapter.get_builder() {
            Some(builer) => {
                builer.get_configurer::<LogoutConfigurer<B>>()
            },
            None => None,
        };
        if let Some(mut logout_configurer) = logout_configurer {
            if !logout_configurer.is_custom_logout_success() {
                logout_configurer.logout_success_url(&format!("{}?logout", url));
            }
        }
    }

    fn set_login_page(&mut self, login_page: impl Into<Box<str>>) {
        self.login_page = login_page.into();
        self.authentication_entry_point = Some(LoginUrlAuthenticationEntryPoint::new(&self.login_page));
    }
}

impl<B, T, F> Required<AbstractHttpConfigurer<T, B>>
    for AbstractAuthenticationFilterConfigurer<B, T, F>
where
    B: HttpSecurityBuilder<B>,
    B: SecurityBuilder<B>,
    T: Required<AbstractAuthenticationFilterConfigurer<B, T, F>>,
    T: Required<AbstractHttpConfigurer<T, B>>,
    F: Required<AbstractAuthenticationProcessingFilter>,
{
    fn get_object(&self) -> &AbstractHttpConfigurer<T, B> {
        &self.abstract_http_configurer
    }

    fn get_object_mut(&mut self) -> &mut AbstractHttpConfigurer<T, B> {
        &mut self.abstract_http_configurer
    }
}

impl<B, T, F> AuthenticationFilterConfigurer<T> for AbstractAuthenticationFilterConfigurer<B, T, F>
where
    B: HttpSecurityBuilder<B>,
    B: SecurityBuilder<B>,
    B: Clone,
    T: Required<AbstractAuthenticationFilterConfigurer<B, T, F>>,
    T: Required<AbstractHttpConfigurer<T, B>>,
    F: Required<AbstractAuthenticationProcessingFilter>,
{
    fn login_processing_url(&mut self, login_processing_url: &str){
        self.login_processing_url = Some(login_processing_url.into());

        let matcher = self.create_login_processing_url_matcher(login_processing_url);
        self.auth_filter.get_object_mut().set_requires_authentication_request_matcher(matcher);
    }

    fn login_page(&mut self, login_page: &str){
        self.set_login_page(login_page);
        self.update_authentication_defaults();
        self.custom_login_page = true;
    }
}

pub trait AuthenticationFilterConfigurer<T> {
    fn login_processing_url(&mut self, login_processing_url: &str);

    fn login_page(&mut self, login_page: &str);

    fn create_login_processing_url_matcher(&self, login_processing_url: &str) -> impl RequestMatcher + 'static {
        AntPathRequestMatcher::from((HttpMethod::Post, login_processing_url))
    }
}
