use std::sync::Arc;

use crate::{
    authorization::authentication_details_source::AuthenticationDetailsSource,
    web::authentication::{
        authentication_failure_handler::AuthenticationFailureHandler,
        authentication_success_handler::AuthenticationSuccessHandler,
        login_url_authentication_entry_point::LoginUrlAuthenticationEntryPoint,
        saved_request_aware_authentication_success_handler::SavedRequestAwareAuthenticationSuccessHandler, simple_url_authentication_failure_handler::SimpleUrlAuthenticationFailureHandler,
    },
};

pub struct AbstractAuthenticationFilterConfigurer<B, T, F> {
    auth_filter: F,

    authentication_details_source: Arc<dyn AuthenticationDetailsSource>,
    default_success_handler: Arc<dyn AuthenticationSuccessHandler>,
    success_handler: Arc<dyn AuthenticationSuccessHandler>,

    authentication_entry_point: LoginUrlAuthenticationEntryPoint,
    custom_login_page: bool,
    login_page: Box<str>,
    login_processing_url: Box<str>,

    failure_handler: Arc<dyn AuthenticationFailureHandler>,
    permit_all: bool,
    failure_url: Option<Box<str>>,

    t: T,
    b: B,
}

impl<B, T, F> AbstractAuthenticationFilterConfigurer<B, T, F> {
    pub fn new(authentication_filter: F, default_login_processing_url: Option<Box<str>>) -> Self {
        let mut configurer = Self {
            auth_filter: authentication_filter,
            success_handler: todo!(),
            authentication_entry_point: todo!(),
            custom_login_page: todo!(),
            login_page: todo!(),
            login_processing_url: todo!(),
            failure_handler: todo!(),
            permit_all: todo!(),
            failure_url: todo!(),
            t: todo!(),
            b: todo!(),
            default_success_handler: todo!(),
        };
        configurer.set_login_page("/login");
        if let Some(url) = default_login_processing_url {
            configurer.login_processing_url(url.as_ref());
        }

        configurer
    }

    fn default_success_url(&mut self, default_success_url: &str, always_use: bool) -> T {
        let mut handler = SavedRequestAwareAuthenticationSuccessHandler::new();
        handler.set_default_target_url(default_success_url);
        handler.set_always_use_default_target_url(always_use);
        self.default_success_handler = Arc::new(handler);
        self.success_handler = self.default_success_handler.clone();
    }

    pub const fn authentication_details_source(
        &self,
        authentication_details_source: Arc<dyn AuthenticationDetailsSource>,
    ) -> T {
        self.authentication_details_source = authentication_details_source;
    }

    pub const fn permit_all(&mut self) -> T {
        self.permit_all = true;
    }

    pub const fn failure_url(&mut self, authentication_failure_url: &str) -> T {
        self.failure_handler(Arc::new(SimpleUrlAuthenticationFailureHandler::new(authentication_failure_url)));
        self.failure_url = Some(authentication_failure_url.into());
    }

    pub const fn failure_handler(
        &mut self,
        authentication_failure_handler: Arc<dyn AuthenticationFailureHandler>,
    ) -> T {
        self.failure_url = None;
        self.failure_handler = authentication_failure_handler;
    }

    fn set_login_page(&mut self, login_page: impl Into<Box<str>>) {
        self.login_page = login_page.into();
        self.authentication_entry_point =
            LoginUrlAuthenticationEntryPoint::new(&self.login_page);
    }
}

impl<B, T, F> AuthenticationFilterConfigurer<T>
    for AbstractAuthenticationFilterConfigurer<B, T, F>
{
    fn login_processing_url(&mut self, login_processing_url: &str) -> T {
        self.login_processing_url = login_processing_url.into();
        self.auth_filter.set
    }
}

pub trait AuthenticationFilterConfigurer<T> {
    fn login_processing_url(&mut self, login_processing_url: &str) -> T;
}
