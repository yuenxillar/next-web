use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{http::HttpMethod, traits::required::Required};

use crate::{
    config::{
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{
            configurers::{
                base_http_configurer::BaseHttpConfigurer, permit_all_support::PermitAllSupport,
                CsrfConfigurer,
            },
            http_security_builder::HttpSecurityBuilder,
        },
    },
    web::{
        authentication::{
            logout::{
                CookieClearingLogoutHandler, DelegatingLogoutSuccessHandler, LogoutFilter,
                LogoutHandler, LogoutSuccessEventPublishingLogoutHandler, LogoutSuccessHandler,
                SecurityContextLogoutHandler, SimpleUrlLogoutSuccessHandler,
            },
            ui::DefaultLoginPageGeneratingFilter,
        },
        context::{HttpSessionSecurityContextRepository, SecurityContextRepository},
        default_security_filter_chain::DefaultSecurityFilterChain,
        util::matcher::{Builder, OrRequestMatcher, RequestMatcher},
    },
};

/// Adds logout support.
#[derive(Clone)]
pub struct LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    logout_handlers: Vec<Arc<dyn LogoutHandler>>,
    context_logout_handler: Option<SecurityContextLogoutHandler>,
    logout_success_url: Option<String>,
    logout_success_handler: Option<Arc<dyn LogoutSuccessHandler>>,
    logout_url: String,
    logout_request_matcher: Option<Arc<dyn RequestMatcher>>,
    permit_all: bool,
    custom_logout_success: bool,
    default_logout_success_handlers: Vec<(Arc<dyn RequestMatcher>, Arc<dyn LogoutSuccessHandler>)>,

    inner: BaseHttpConfigurer<Self, H>,
}

impl<H> LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    /// Adds a LogoutHandler. SecurityContextLogoutHandler and
    /// LogoutSuccessEventPublishingLogoutHandler are added as last LogoutHandler instances by default.
    pub fn add_logout_handler(&mut self, logout_handler: Arc<dyn LogoutHandler>) -> &mut Self {
        self.logout_handlers.push(logout_handler);

        self
    }

    /// Specifies if SecurityContextLogoutHandler should clear the Authentication at the time of logout.
    pub fn clear_authentication(&mut self, clear_authentication: bool) -> &mut Self {
        self.context_logout_handler
            .as_mut()
            .map(|h| h.set_clear_authentication(clear_authentication));

        self
    }

    /// Causes the HttpSession to be invalidated when this LogoutHandler is invoked. Defaults to true.
    pub fn invalidate_http_session(&mut self, invalidate_http_session: bool) -> &mut Self {
        self.context_logout_handler
            .as_mut()
            .map(|h| h.set_invalidate_http_session(invalidate_http_session));

        self
    }

    ///The URL that triggers log out to occur (default is "/logout"). If CSRF protection is enabled (default),
    /// then the request must also be a POST. This means that by default POST "/logout" is required to trigger a log out.
    ///  If CSRF protection is disabled, then any HTTP method is allowed.
    pub fn logout_url(&mut self, logout_url: impl Into<String>) -> &mut Self {
        self.logout_request_matcher = None;
        self.logout_url = logout_url.into();

        self
    }

    /// The RequestMatcher that triggers log out to occur.
    /// In most circumstances users will use logout_url(String) which helps enforce good practices.
    pub fn logout_request_matcher(
        &mut self,
        logout_request_matcher: Arc<dyn RequestMatcher>,
    ) -> &mut Self {
        self.logout_request_matcher = Some(logout_request_matcher);

        self
    }

    /// The URL to redirect to after logout has occurred. The default is "/login?logout". This is a shortcut
    /// for invoking logoutSuccessHandler(LogoutSuccessHandler) with a SimpleUrlLogoutSuccessHandler.
    pub fn logout_success_url(&mut self, logout_success_url: impl Into<String>) -> &mut Self {
        self.custom_logout_success = true;
        self.logout_success_url = Some(logout_success_url.into());

        self
    }

    /// Grants access to the logoutSuccessUrl(String) and the logoutUrl(String) for every user.
    pub fn permit_all(&mut self, permit_all: bool) -> &mut Self {
        self.permit_all = permit_all;

        self
    }

    /// Allows specifying the names of cookies to be removed on logout success.
    /// This is a shortcut to easily invoke addLogoutHandler(LogoutHandler) with a CookieClearingLogoutHandler.
    pub fn delete_cookies<T, I>(&mut self, cookie_names_to_clear: T) -> &mut Self
    where
        T: IntoIterator<Item = I>,
        I: ToString,
    {
        self.add_logout_handler(Arc::new(CookieClearingLogoutHandler::new(
            cookie_names_to_clear
                .into_iter()
                .map(|name| name.to_string())
                .collect(),
        )));

        self
    }

    /// Sets the LogoutSuccessHandler to use. If this is specified, logoutSuccessUrl(String) is ignored.
    pub fn logout_success_handler<T>(&mut self, logout_success_handler: T) -> &mut Self
    where
        T: LogoutSuccessHandler,
        T: 'static,
    {
        self.logout_success_url = None;
        self.custom_logout_success = true;
        self.logout_success_handler = Some(Arc::new(logout_success_handler));

        self
    }

    pub fn default_logout_success_handler_for<T>(
        &mut self,
        logout_success_handler: T,
        preferred_matcher: Arc<dyn RequestMatcher>,
    ) -> &mut Self
    where
        T: LogoutSuccessHandler,
        T: 'static,
    {
        self.default_logout_success_handlers
            .push((preferred_matcher, Arc::new(logout_success_handler)));

        self
    }

    /// Gets the LogoutSuccessHandler if not null, otherwise
    /// creates a new SimpleUrlLogoutSuccessHandler using the logoutSuccessUrl(String).
    pub fn get_logout_success_handler(&mut self) -> &Arc<dyn LogoutSuccessHandler> {
        if self.logout_success_handler.is_none() {
            let handler = self.create_default_success_handler();
            self.logout_success_handler = Some(handler);
        }

        self.logout_success_handler.as_ref().unwrap()
    }

    fn create_default_success_handler(&self) -> Arc<dyn LogoutSuccessHandler> {
        let mut url_logout_handler = SimpleUrlLogoutSuccessHandler::default();
        url_logout_handler
            .set_default_target_url(self.logout_success_url.as_deref().unwrap_or_default());
        if self.default_logout_success_handlers.is_empty() {
            return Arc::new(url_logout_handler);
        }

        let mut success_handler =
            DelegatingLogoutSuccessHandler::new(self.default_logout_success_handlers.clone());
        success_handler.set_default_logout_success_handler(Arc::new(url_logout_handler));

        Arc::new(success_handler)
    }

    /// Returns true if the logout success has been customized via logoutSuccessUrl(String) or logoutSuccessHandler(LogoutSuccessHandler).
    pub fn is_custom_logout_success(&self) -> bool {
        self.custom_logout_success
    }

    /// Gets the logoutSuccessUrl or null if a logoutSuccessHandler(LogoutSuccessHandler) was configured.
    pub fn get_logout_success_url(&self) -> Option<&str> {
        self.logout_success_url.as_deref()
    }

    /// Returns the registered logout handlers.
    pub fn get_logout_handlers(&self) -> &[Arc<dyn LogoutHandler>] {
        &self.logout_handlers
    }

    /// Creates the LogoutFilter using the LogoutHandler instances,
    /// the logoutSuccessHandler(LogoutSuccessHandler) and the logoutUrl(String).
    fn create_logout_filter(&mut self, http: &mut H) -> LogoutFilter {
        if let Some(mut context_logout_handler) = self.context_logout_handler.take() {
            context_logout_handler.set_security_context_holder_strategy(
                self.inner.get_security_context_holder_strategy().to_owned(),
            );
            context_logout_handler
                .set_security_context_repository(self.get_security_context_repository(http));
            self.logout_handlers.push(Arc::new(context_logout_handler));
        }
        self.logout_handlers.push(Arc::new(
            LogoutSuccessEventPublishingLogoutHandler::default(),
        ));

        let handlers = std::mem::take(&mut self.logout_handlers);
        let logout_success_handler = self.get_logout_success_handler().to_owned();
        let mut filter = LogoutFilter::new(logout_success_handler, handlers);

        filter.set_security_context_holder_strategy(
            self.inner.get_security_context_holder_strategy().to_owned(),
        );
        filter.set_logout_request_matcher(self.get_logout_request_matcher(http));

        filter
    }

    fn get_security_context_repository(&self, http: &H) -> Arc<dyn SecurityContextRepository> {
        match http.shared_object::<Arc<dyn SecurityContextRepository>>() {
            Some(repo) => repo.clone(),
            None => Arc::new(HttpSessionSecurityContextRepository::default()),
        }
    }

    fn get_logout_request_matcher(&self, http: &H) -> Arc<dyn RequestMatcher> {
        match self.logout_request_matcher.as_ref() {
            Some(rm) => rm.clone(),
            None => self.create_logout_request_matcher(http),
        }
    }

    fn create_logout_request_matcher(&self, http: &H) -> Arc<dyn RequestMatcher> {
        http.shared_object::<Builder>()
            .map(|builder| {
                let post = builder.matcher(Some(HttpMethod::POST), &self.logout_url);
                if http.configurer::<CsrfConfigurer<H>>().is_some() {
                    return Arc::new(post) as Arc<dyn RequestMatcher>;
                }

                let get = builder.matcher(Some(HttpMethod::GET), &self.logout_url);
                let put = builder.matcher(Some(HttpMethod::PUT), &self.logout_url);
                let delete = builder.matcher(Some(HttpMethod::DELETE), &self.logout_url);

                let ele = [get, post, put, delete]
                    .into_iter()
                    .map(|a| Arc::new(a) as Arc<dyn RequestMatcher>)
                    .collect::<Vec<_>>();
                Arc::new(OrRequestMatcher::new(ele))
            })
            .unwrap()
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>> for LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: crate::config::security_builder::SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.inner.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.inner.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    fn init(&mut self, http: &mut H) {
        if self.permit_all {
            PermitAllSupport::permit_all(
                http,
                &[self.logout_success_url.as_deref().unwrap_or_default()],
            );
            PermitAllSupport::permit_all_with_matcher(
                http,
                &[self.get_logout_request_matcher(http)],
            );
        }

        if let Some(login_page_generating_filter) =
            http.shared_object_mut::<DefaultLoginPageGeneratingFilter>()
        {
            if !self.is_custom_logout_success() {
                self.get_logout_success_url()
                    .map(|s| login_page_generating_filter.set_logout_success_url(s));
            }
        }
    }

    fn configure(&mut self, http: &mut H) {
        let filter = self.create_logout_filter(http);
        http.add_filter(filter);
    }
}

impl<H> Default for LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            logout_handlers: Default::default(),
            context_logout_handler: Some(Default::default()),
            logout_success_url: Some(String::from("/login?logout")),
            logout_success_handler: None,
            logout_url: String::from("/logout"),
            logout_request_matcher: None,
            permit_all: false,
            custom_logout_success: false,
            default_logout_success_handlers: Default::default(),

            inner: Default::default(),
        }
    }
}

impl<H> Deref for LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<H> DerefMut for LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
