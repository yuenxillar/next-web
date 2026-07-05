use std::{marker::PhantomData, sync::Arc};

use next_web_core::{traits::required::Required, util::http_method::HttpMethod};

use crate::{
    authorization::AuthenticationDetailsSource,
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        web::{
            configurers::{
                base_http_configurer::BaseHttpConfigurer, permit_all_support::PermitAllSupport,
            },
            http_security_builder::HttpSecurityBuilder,
        },
    },
    web::util::matcher::{AntPathRequestMatcher, RequestMatcher},
    web::{
        authentication::{
            authentication_failure_handler::AuthenticationFailureHandler,
            authentication_success_handler::AuthenticationSuccessHandler,
            base_authentication_processing_filter::BaseAuthenticationProcessingFilter,
            login_url_authentication_entry_point::LoginUrlAuthenticationEntryPoint,
            saved_request_aware_authentication_success_handler::SavedRequestAwareAuthenticationSuccessHandler,
            simple_url_authentication_failure_handler::SimpleUrlAuthenticationFailureHandler,
        },
        default_security_filter_chain::DefaultSecurityFilterChain,
        AuthenticationEntryPoint,
    },
};

#[derive(Clone)]
pub struct BaseAuthenticationFilterConfigurer<B, T, F>
where
    B: HttpSecurityBuilder<B>,
    T: Required<Self>,
    T: Required<BaseHttpConfigurer<T, B>>,
    F: Required<BaseAuthenticationProcessingFilter>,
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

    pub(super) base_http_configurer: BaseHttpConfigurer<T, B>,

    _marker_1: PhantomData<T>,
    _marker_2: PhantomData<B>,
}

impl<B, T, F> BaseAuthenticationFilterConfigurer<B, T, F>
where
    B: HttpSecurityBuilder<B>,
    B: Clone,
    T: Required<BaseAuthenticationFilterConfigurer<B, T, F>>,
    T: Required<BaseHttpConfigurer<T, B>>,
    F: Required<BaseAuthenticationProcessingFilter>,
{
    pub fn new(authentication_filter: F, default_login_processing_url: Option<Box<str>>) -> Self {
        let default_success_handler =
            Arc::new(SavedRequestAwareAuthenticationSuccessHandler::new());
        let mut authentication_filter = authentication_filter;
        authentication_filter
            .get_mut_object()
            .set_success_handler(default_success_handler.clone());
        let mut configurer = Self {
            default_success_handler: default_success_handler.clone(),
            auth_filter: authentication_filter,
            success_handler: default_success_handler,

            base_http_configurer: BaseHttpConfigurer::default(),
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

        configurer
    }

    pub fn authentication_details_source(
        &mut self,
        authentication_details_source: Arc<dyn AuthenticationDetailsSource>,
    ) {
        self.authentication_details_source = Some(authentication_details_source);
    }

    pub fn success_handler<T1>(&mut self, success_handler: T1)
    where
        T1: AuthenticationSuccessHandler + 'static,
    {
        self.success_handler = Arc::new(success_handler);
        self.auth_filter
            .get_mut_object()
            .set_success_handler(self.success_handler.clone());
    }

    pub const fn permit_all(&mut self) {
        self.permit_all = true;
    }

    pub fn failure_url(&mut self, authentication_failure_url: &str) {
        self.failure_handler(SimpleUrlAuthenticationFailureHandler::new(
            authentication_failure_url,
        ));
        self.failure_url = Some(authentication_failure_url.into());
    }

    pub fn failure_handler<T1>(&mut self, authentication_failure_handler: T1)
    where
        T1: AuthenticationFailureHandler + 'static,
    {
        self.failure_url = None;
        self.failure_handler = Some(Arc::new(authentication_failure_handler));
        self.auth_filter
            .get_mut_object()
            .set_failure_handler(self.failure_handler.clone().unwrap());
    }

    pub fn is_custom_login_page(&self) -> bool {
        self.custom_login_page
    }

    pub fn get_authentication_filter(&self) -> &F {
        &self.auth_filter
    }

    pub fn get_mut_authentication_filter(&mut self) -> &mut F {
        &mut self.auth_filter
    }

    pub fn set_authentication_filter(&mut self, auth_filter: F) {
        self.auth_filter = auth_filter;
    }

    pub fn get_login_page(&self) -> &str {
        self.login_page.as_ref()
    }

    pub fn get_authentication_entry_point(&self) -> Option<&dyn AuthenticationEntryPoint> {
        self.authentication_entry_point
            .as_ref()
            .map(|ep| ep as &dyn AuthenticationEntryPoint)
    }

    pub fn get_login_processing_url(&self) -> Option<&str> {
        self.login_processing_url.as_deref()
    }

    pub fn get_failure_url(&self) -> Option<&str> {
        self.failure_url.as_deref()
    }

    pub fn update_authentication_defaults(&mut self) {
        // let url = self.login_page.to_string();
        // if self.login_processing_url.is_none() {
        //     self.login_processing_url(&url);
        // }

        // if self.failure_handler.is_none() {
        //     self.failure_url(&format!("{}?error", url));
        // }

        // let abstract_http_configurer = self.get_object();
        // let security_configurer_adapter = abstract_http_configurer.get_object();

        // let logout_configurer = match security_configurer_adapter.get_builder() {
        //     Some(builer) => builer.get_configurer::<LogoutConfigurer<B>>(),
        //     None => None,
        // };
        // if let Some(mut logout_configurer) = logout_configurer {
        //     if !logout_configurer.is_custom_logout_success() {
        //         logout_configurer.logout_success_url(&format!("{}?logout", url));
        //     }
        // }
        //
        todo!()
    }

    pub fn update_access_defaults(&mut self, http: &mut B)
    where
        B: 'static,
    {
        if self.permit_all {
            let urls = vec![
                self.login_page.as_ref(),
                self.login_processing_url.as_deref().unwrap_or_default(),
                self.failure_url.as_deref().unwrap_or_default(),
            ];
            PermitAllSupport::permit_all(http, &urls);
        }
    }

    fn set_login_page(&mut self, login_page: impl Into<Box<str>>) {
        self.login_page = login_page.into();
        self.authentication_entry_point =
            Some(LoginUrlAuthenticationEntryPoint::new(&self.login_page));
    }
}

impl<B, T, F> Required<BaseHttpConfigurer<T, B>> for BaseAuthenticationFilterConfigurer<B, T, F>
where
    B: HttpSecurityBuilder<B>,
    T: Required<BaseAuthenticationFilterConfigurer<B, T, F>>,
    T: Required<BaseHttpConfigurer<T, B>>,
    F: Required<BaseAuthenticationProcessingFilter>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<T, B> {
        &self.base_http_configurer
    }

    fn get_mut_object(&mut self) -> &mut BaseHttpConfigurer<T, B> {
        &mut self.base_http_configurer
    }
}

impl<B, T, F> AuthenticationFilterConfigurer<T> for BaseAuthenticationFilterConfigurer<B, T, F>
where
    B: HttpSecurityBuilder<B>,
    B: Clone,
    T: Required<BaseAuthenticationFilterConfigurer<B, T, F>>,
    T: Required<BaseHttpConfigurer<T, B>>,
    F: Required<BaseAuthenticationProcessingFilter>,
{
    fn login_processing_url(&mut self, login_processing_url: &str) {
        self.login_processing_url = Some(login_processing_url.into());

        let matcher = self.create_login_processing_url_matcher(login_processing_url);
        self.auth_filter
            .get_mut_object()
            .set_requires_authentication_request_matcher(matcher);
    }

    fn login_page(&mut self, login_page: &str) {
        self.set_login_page(login_page);
        self.update_authentication_defaults();
        self.custom_login_page = true;
    }
}

impl<B, T, F> SecurityConfigurer<DefaultSecurityFilterChain, B>
    for BaseAuthenticationFilterConfigurer<B, T, F>
where
    B: HttpSecurityBuilder<B>,
    B: SecurityBuilder<DefaultSecurityFilterChain>,
    B: Clone,
    B: 'static,
    T: Required<BaseAuthenticationFilterConfigurer<B, T, F>>,
    T: Required<BaseHttpConfigurer<T, B>>,
    T: Sync + Send,
    F: Required<BaseAuthenticationProcessingFilter> + Clone + Sync + Send,
    F: next_web_core::traits::filter::HttpFilter,
{
    fn init(&mut self, http: &mut B) {
        self.update_authentication_defaults();
        self.update_access_defaults(http);
    }

    fn configure(&mut self, http: &mut B) {
        // Add the authentication filter to the chain.
        let filter = self.auth_filter.clone();
        http.add_filter(filter);
    }
}

pub trait AuthenticationFilterConfigurer<T> {
    fn login_processing_url(&mut self, login_processing_url: &str);

    fn login_page(&mut self, login_page: &str);
    fn create_login_processing_url_matcher(
        &self,
        login_processing_url: &str,
    ) -> impl RequestMatcher + 'static {
        AntPathRequestMatcher::from((HttpMethod::Post, login_processing_url))
    }
}
