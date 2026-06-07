use std::{marker::PhantomData, sync::Arc};

use next_web_core::traits::required::Required;

use crate::{
    config::{
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{
            configurers::base_http_configurer::BaseHttpConfigurer,
            http_security_builder::HttpSecurityBuilder,
        },
    },
    web::{
        authentication::{
            logout::{LogoutFilter, LogoutHandler, SimpleUrlLogoutSuccessHandler},
            ui::default_login_page_generating_filter::DefaultLoginPageGeneratingFilter,
        },
        default_security_filter_chain::DefaultSecurityFilterChain,
    },
};

#[derive(Clone)]
pub struct LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<LogoutConfigurer<H>, H>>,
{
    _marker: PhantomData<H>,
    logout_success_url: Option<Box<str>>,

    logout_handlers: Vec<Arc<dyn LogoutHandler>>,
    base_http_configurer: BaseHttpConfigurer<LogoutConfigurer<H>, H>,
}

impl<H> LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<LogoutConfigurer<H>, H>>,
{
    pub fn is_custom_logout_success(&self) -> bool {
        self.logout_success_url.is_some()
    }

    pub fn logout_success_url(&mut self, logout_success_url: &str) {
        self.logout_success_url = Some(logout_success_url.into());
    }

    pub fn get_logout_success_url(&self) -> Option<&str> {
        self.logout_success_url.as_deref()
    }

    /// Returns the registered logout handlers.
    pub fn get_logout_handlers(&self) -> &[Arc<dyn LogoutHandler>] {
        &self.logout_handlers
    }

    pub fn add_logout_handler<T>(&mut self, logout_handler: T)
    where
        T: LogoutHandler,
        T: 'static,
    {
        self.logout_handlers.push(Arc::new(logout_handler));
    }
}

impl<H> Default for LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<LogoutConfigurer<H>, H>>,
{
    fn default() -> Self {
        Self {
            _marker: PhantomData,
            logout_handlers: Default::default(),
            logout_success_url: Default::default(),
            base_http_configurer: Default::default(),
        }
    }
}

impl<H> Required<BaseHttpConfigurer<LogoutConfigurer<H>, H>> for LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<LogoutConfigurer<H>, H> {
        &self.base_http_configurer
    }

    fn get_mut_object(&mut self) -> &mut BaseHttpConfigurer<LogoutConfigurer<H>, H> {
        &mut self.base_http_configurer
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>> for LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: crate::config::security_builder::SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base_http_configurer.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base_http_configurer.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {}

    fn configure(&mut self, http: &mut H) {
        // Wire up DefaultLoginPageGeneratingFilter if present
        if let Some(login_page_filter) =
            http.get_mut_shared_object::<DefaultLoginPageGeneratingFilter>()
        {
            if let Some(url) = &self.logout_success_url {
                login_page_filter.set_logout_success_url(url.clone());
            }
        }

        // Create and add the LogoutFilter
        let handlers = self.logout_handlers.clone();
        let filter = if let Some(success_url) = &self.logout_success_url {
            LogoutFilter::with_logout_success_url(success_url.as_ref(), handlers)
        } else {
            let mut handler = SimpleUrlLogoutSuccessHandler::default();
            handler.set_default_target_url("/login?logout");
            LogoutFilter::new(Arc::new(handler), handlers)
        };
        http.add_filter(filter);
    }
}
