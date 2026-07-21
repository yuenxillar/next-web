use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{
    http::MediaType,
    traits::filter::HttpFilter,
    util::http_method::HttpMethod,
    web::accept::{ContentNegotiationStrategy, HeaderContentNegotiationStrategy},
};

use crate::{
    authorization::{AuthenticationDetailsSource, AuthenticationManager},
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        web::{
            configurers::{
                permit_all_support::PermitAllSupport, BaseHttpConfigurer, ErrorHandlingConfigurer,
                LogoutConfigurer, SecurityContextConfigurer,
            },
            http_security_builder::HttpSecurityBuilder,
        },
    },
    web::{
        authentication::{
            authentication_failure_handler::AuthenticationFailureHandler,
            base_authentication_processing_filter::BaseAuthenticationProcessingFilter,
            login_url_authentication_entry_point::LoginUrlAuthenticationEntryPoint,
            session::SessionAuthenticationStrategy,
            simple_url_authentication_failure_handler::SimpleUrlAuthenticationFailureHandler,
            AuthenticationSuccessHandler, RememberMeServices,
            SavedRequestAwareAuthenticationSuccessHandler,
        },
        context::SecurityContextRepository,
        default_security_filter_chain::DefaultSecurityFilterChain,
        savedrequest::RequestCache,
        util::matcher::{
            AndRequestMatcher, AntPathRequestMatcher, MediaTypeRequestMatcher,
            NegatedRequestMatcher, RequestHeaderRequestMatcher, RequestMatcher,
        },
        AuthenticationEntryPoint, PortMapper,
    },
};

/// Base class for configuring BaseAuthenticationFilterConfigurer. This is intended for internal use only.
#[derive(Clone)]
pub struct BaseAuthenticationFilterConfigurer<B, T, F>
where
    B: HttpSecurityBuilder<B>,

    T: Deref<Target = Self>,
    T: DerefMut,
    F: Deref<Target = BaseAuthenticationProcessingFilter>,
    F: DerefMut,
{
    pub(crate) auth_filter: Option<F>,
    authentication_details_source: Option<Arc<dyn AuthenticationDetailsSource>>,
    default_success_handler: SavedRequestAwareAuthenticationSuccessHandler,
    success_handler: Arc<dyn AuthenticationSuccessHandler>,
    authentication_entry_point: Option<LoginUrlAuthenticationEntryPoint>,
    custom_login_page: bool,
    login_page: Box<str>,
    pub(crate) login_processing_url: Option<Box<str>>,
    failure_handler: Option<Arc<dyn AuthenticationFailureHandler>>,
    permit_all: bool,
    failure_url: Option<Box<str>>,

    pub(super) inner: BaseHttpConfigurer<T, B>,
}

impl<B, T, F> BaseAuthenticationFilterConfigurer<B, T, F>
where
    B: HttpSecurityBuilder<B>,

    T: Deref<Target = Self>,
    T: DerefMut,
    F: Deref<Target = BaseAuthenticationProcessingFilter>,
    F: DerefMut,
{
    pub fn new(authentication_filter: F, default_login_processing_url: Option<&str>) -> Self {
        let mut configurer = Self::default();
        configurer.set_authentication_filter(authentication_filter);
        if let Some(url) = default_login_processing_url {
            configurer.login_processing_url(url);
        }

        configurer
    }

    /// Specifies where users will be redirected after authenticating successfully if they
    /// have not visited a secured page prior to authenticating. This is a shortcut for
    /// calling `default_success_url(url, false)`.
    pub fn default_success_url(&mut self, default_success_url: &str) -> &mut Self {
        self.default_success_url_with_always(default_success_url, false)
    }

    /// Specifies where users will be redirected after authenticating successfully if they
    /// have not visited a secured page prior to authenticating or `always_use` is
    /// true. This is a shortcut for calling `success_handler`.
    pub fn default_success_url_with_always(
        &mut self,
        default_success_url: &str,
        always_use: bool,
    ) -> &mut Self {
        let mut handler = SavedRequestAwareAuthenticationSuccessHandler::default();
        handler.set_default_target_url(default_success_url);
        handler.set_always_use_default_target_url(always_use);
        self.default_success_handler = handler.to_owned();
        self.success_handler(Arc::new(handler) as Arc<dyn AuthenticationSuccessHandler>);

        self
    }

    /// Specifies the `SecurityContextRepository` to use.
    pub fn security_context_repository(
        &mut self,
        security_context_repository: Arc<dyn SecurityContextRepository>,
    ) -> &mut Self {
        self.auth_filter
            .as_mut()
            .map(|filter| filter.set_security_context_repository(security_context_repository));

        self
    }

    /// Specifies a custom `AuthenticationDetailsSource`. The default is
    /// `WebAuthenticationDetailsSource`.
    ///

    pub fn authentication_details_source(
        &mut self,
        authentication_details_source: Arc<dyn AuthenticationDetailsSource>,
    ) {
        self.authentication_details_source = Some(authentication_details_source);
    }

    /// Specifies the `AuthenticationSuccessHandler` to be used. The default is
    /// `SavedRequestAwareAuthenticationSuccessHandler` with no additional properties
    /// set.
    pub fn success_handler(&mut self, success_handler: Arc<dyn AuthenticationSuccessHandler>) {
        self.success_handler = success_handler;
        self.auth_filter
            .as_mut()
            .map(|filter| filter.set_success_handler(self.success_handler.clone()));
    }

    /// Ensures the urls for failureUrl(String) as well as for the HttpSecurityBuilder,
    /// the getLoginPage and getLoginProcessingUrl are granted access to any user.
    pub fn permit_all(&mut self, permit_all: bool) {
        self.permit_all = permit_all;
    }

    /// The URL to send users if authentication fails. This is a shortcut for invoking
    /// `failure_handler`. The default is `"/login?error"`
    pub fn failure_url(&mut self, authentication_failure_url: &str) {
        self.failure_handler(Arc::new(SimpleUrlAuthenticationFailureHandler::new(
            authentication_failure_url,
        )));
        self.failure_url = Some(authentication_failure_url.into());
    }

    /// Specifies the `AuthenticationFailureHandler` to use when authentication
    /// fails. The default is redirecting to `"/login?error"` using
    /// `SimpleUrlAuthenticationFailureHandler`.
    pub fn failure_handler(
        &mut self,
        authentication_failure_handler: Arc<dyn AuthenticationFailureHandler>,
    ) {
        self.failure_url = None;

        self.failure_handler = Some(authentication_failure_handler.clone());
        self.auth_filter
            .as_mut()
            .map(|filter| filter.set_failure_handler(authentication_failure_handler));
    }

    /// Specifies the URL to send users to if login is required. If used with EnableWebSecurity a default login page
    /// will be generated when this attribute is not specified.
    ///
    /// If a URL is specified or this is not being used in conjunction with EnableWebSecurity,
    /// users are required to process the specified URL to generate a login page.
    pub fn login_page(&mut self, login_page: impl Into<Box<str>>) {
        self.set_login_page(login_page);
        self.update_authentication_defaults(todo!());
        self.custom_login_page = true;
    }

    /// Registers the default authentication entry point.
    ///
    /// # Arguments
    ///
    /// * `http` - the `HttpSecurityBuilder` to use
    pub fn register_default_authentication_entry_point(&mut self, http: &mut B)
    where
        B: 'static,
    {
        if let Some(entry_point) = self.authentication_entry_point.as_ref() {
            self.register_authentication_entry_point(http, Arc::new(entry_point.clone()));
        }
    }

    /// Registers an authentication entry point.
    ///
    /// # Arguments
    ///
    /// * `http` - the `HttpSecurityBuilder` to use
    /// * `authentication_entry_point` - the entry point to register
    pub fn register_authentication_entry_point(
        &mut self,
        http: &mut B,
        authentication_entry_point: Arc<dyn AuthenticationEntryPoint>,
    ) where
        B: 'static,
    {
        let exists = http.configurer::<ErrorHandlingConfigurer<B>>();
        if exists.is_some() {
            // let processed_entry_point = self.post_process(authentication_entry_point);
            let matcher = self.get_authentication_entry_point_matcher(http);

            if let Some(error_handling) = http.configurer_mut::<ErrorHandlingConfigurer<B>>() {
                error_handling
                    .default_authentication_entry_point_for(authentication_entry_point, matcher);
            }
        }
    }

    /// Gets the authentication entry point matcher.
    ///
    /// # Arguments
    ///
    /// * `http` - the `HttpSecurityBuilder` to use
    ///
    /// # Returns
    ///
    /// The `RequestMatcher` for the authentication entry point
    pub fn get_authentication_entry_point_matcher(&self, http: &B) -> Arc<dyn RequestMatcher> {
        let content_negotiation_strategy = http
            .shared_object::<Arc<dyn ContentNegotiationStrategy>>()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| Arc::new(HeaderContentNegotiationStrategy::default()));

        let mut media_matcher = MediaTypeRequestMatcher::new(
            content_negotiation_strategy,
            vec![
                MediaType::application_xhtml_xml(),
                MediaType::with_subtype("image", "*"),
                MediaType::text_html(),
                MediaType::text_plain(),
            ],
        );
        media_matcher.set_ignored_media_types(vec![MediaType::all()]);

        let not_x_requested_with = Arc::new(NegatedRequestMatcher::new(
            RequestHeaderRequestMatcher::new("X-Requested-With", Some("XMLHttpRequest".into())),
        ));

        Arc::new(AndRequestMatcher::new(vec![
            not_x_requested_with,
            Arc::new(media_matcher),
        ]))
    }

    /// Return true if a custom login page has been specified, else false
    pub fn is_custom_login_page(&self) -> bool {
        self.custom_login_page
    }

    /// Gets the Authentication Filter.
    pub fn get_authentication_filter(&self) -> Option<&F> {
        self.auth_filter.as_ref()
    }

    /// Gets the Authentication Filter (mutable).
    pub fn get_authentication_filter_mut(&mut self) -> Option<&mut F> {
        self.auth_filter.as_mut()
    }

    /// Sets the Authentication Filter.
    pub fn set_authentication_filter(&mut self, auth_filter: F) {
        self.auth_filter = Some(auth_filter);
    }

    /// Gets the login page.
    pub fn get_login_page(&self) -> &str {
        self.login_page.as_ref()
    }

    /// Gets the Authentication Entry Point.
    pub fn get_authentication_entry_point(&self) -> Option<&LoginUrlAuthenticationEntryPoint> {
        self.authentication_entry_point.as_ref()
    }

    /// Gets the URL to submit an authentication request to (i.e. where username/password
    /// must be submitted).
    pub fn get_login_processing_url(&self) -> Option<&str> {
        self.login_processing_url.as_deref()
    }

    /// Gets the URL to send users to if authentication fails.
    pub fn get_failure_url(&self) -> Option<&str> {
        self.failure_url.as_deref()
    }

    /// Updates the default values for authentication.
    pub fn update_authentication_defaults(&mut self, http: &mut B)
    where
        B: 'static,
    {
        if let Some(s) = self.login_processing_url.to_owned() {
            self.login_processing_url(s.as_ref());
        }
        if self.failure_handler.is_none() {
            self.failure_url(&format!("{}?error", self.login_page));
        }
        if let Some(logout_configurer) = http.configurer_mut::<LogoutConfigurer<B>>() {
            if !logout_configurer.is_custom_logout_success() {
                logout_configurer.logout_success_url(&format!("{}?logout", &self.login_page));
            }
        }
    }

    /// Updates the default values for access.
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

    /// Sets the login_page and updates the `AuthenticationEntryPoint`.
    fn set_login_page(&mut self, login_page: impl Into<Box<str>>) {
        self.login_page = login_page.into();
        self.authentication_entry_point =
            Some(LoginUrlAuthenticationEntryPoint::new(&self.login_page));
    }
}

impl<B, T, F> Deref for BaseAuthenticationFilterConfigurer<B, T, F>
where
    B: HttpSecurityBuilder<B>,

    T: Deref<Target = Self>,
    T: DerefMut,
    F: Deref<Target = BaseAuthenticationProcessingFilter>,
    F: DerefMut,
{
    type Target = BaseHttpConfigurer<T, B>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<B, T, F> DerefMut for BaseAuthenticationFilterConfigurer<B, T, F>
where
    B: HttpSecurityBuilder<B>,

    T: Deref<Target = Self>,
    T: DerefMut,
    F: Deref<Target = BaseAuthenticationProcessingFilter>,
    F: DerefMut,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<B, T, F> BaseAuthenticationFilterConfigurerExt for BaseAuthenticationFilterConfigurer<B, T, F>
where
    B: HttpSecurityBuilder<B>,

    T: Deref<Target = Self>,
    T: DerefMut,
    F: Deref<Target = BaseAuthenticationProcessingFilter>,
    F: DerefMut,
{
    fn login_processing_url(&mut self, login_processing_url: &str) {
        self.login_processing_url = Some(login_processing_url.into());

        let matcher = self.create_login_processing_url_matcher(login_processing_url);
        self.auth_filter
            .as_mut()
            .map(|filter| filter.set_requires_authentication_request_matcher(matcher));
    }

    fn login_page(&mut self, login_page: &str) {
        self.set_login_page(login_page);
        self.update_authentication_defaults(todo!());
        self.custom_login_page = true;
    }
}

impl<B, T, F> SecurityConfigurer<DefaultSecurityFilterChain, B>
    for BaseAuthenticationFilterConfigurer<B, T, F>
where
    B: HttpSecurityBuilder<B>,
    B: SecurityBuilder<DefaultSecurityFilterChain>,
    B: 'static,

    T: Deref<Target = Self>,
    T: DerefMut,
    T: Send + Sync,
    F: Deref<Target = BaseAuthenticationProcessingFilter>,
    F: DerefMut,
    F: HttpFilter,
{
    fn init(&mut self, http: &mut B) {
        self.update_authentication_defaults(http);
        self.update_access_defaults(http);
        self.register_default_authentication_entry_point(http);
    }

    fn configure(&mut self, http: &mut B) {
        // Configure port mapper
        if let Some(port_mapper) = http.shared_object::<Arc<dyn PortMapper>>() {
            self.authentication_entry_point
                .as_mut()
                .map(|luaep| luaep.set_port_mapper(port_mapper.to_owned()));
        }

        // Configure request cache
        if let Some(request_cache) = http.shared_object::<Arc<dyn RequestCache>>() {
            self.default_success_handler
                .set_request_cache(request_cache.to_owned());
            self.success_handler = Arc::new(self.default_success_handler.to_owned());
        }

        // Set authentication manager
        if let Some(auth_manager) = http.shared_object::<Arc<dyn AuthenticationManager>>() {
            self.auth_filter
                .as_mut()
                .map(|filter| filter.set_authentication_manager(auth_manager.to_owned()));
        }

        // Set success handler
        self.auth_filter.as_mut().map(|filter| {
            filter.set_authentication_success_handler(self.success_handler.to_owned())
        });

        // Set failure handler if configured
        if let Some(failure_handler) = self.failure_handler.as_ref() {
            self.auth_filter.as_mut().map(|filter| {
                filter.set_authentication_failure_handler(failure_handler.to_owned())
            });
        }

        // Set authentication details source if configured
        if let Some(details_source) = self.authentication_details_source.as_ref() {
            self.auth_filter
                .as_mut()
                .map(|filter| filter.set_authentication_details_source(details_source.to_owned()));
        }

        // Configure session authentication strategy
        if let Some(session_strategy) =
            http.shared_object::<Arc<dyn SessionAuthenticationStrategy>>()
        {
            self.auth_filter.as_mut().map(|filter| {
                filter.set_session_authentication_strategy(session_strategy.to_owned())
            });
        }

        // Configure remember-me services
        if let Some(remember_me_services) = http.shared_object::<Arc<dyn RememberMeServices>>() {
            self.auth_filter
                .as_mut()
                .map(|filter| filter.set_remember_me_services(remember_me_services.to_owned()));
        }

        // Configure security context repository if required
        if let Some(security_context_configurer) = http.configurer::<SecurityContextConfigurer<B>>()
        {
            if security_context_configurer.is_require_explicit_save() {
                let repo = security_context_configurer.get_security_context_repository(http);
                self.auth_filter
                    .as_mut()
                    .map(|filter| filter.set_security_context_repository(repo));
            }
        }

        // Set security context holder strategy
        if let Some(mut auth_filter) = self.auth_filter.take() {
            auth_filter.set_security_context_holder_strategy(
                self.get_security_context_holder_strategy().to_owned(),
            );

            // self.post_process(auth_filter);
            http.add_filter(auth_filter);
        }
    }
}

impl<B, T, F> Default for BaseAuthenticationFilterConfigurer<B, T, F>
where
    B: HttpSecurityBuilder<B>,

    T: Deref<Target = Self>,
    T: DerefMut,
    F: Deref<Target = BaseAuthenticationProcessingFilter>,
    F: DerefMut,
{
    fn default() -> Self {
        let default_success_handler = SavedRequestAwareAuthenticationSuccessHandler::default();
        let mut configurer = Self {
            default_success_handler: default_success_handler.to_owned(),
            success_handler: Arc::new(default_success_handler),
            auth_filter: None,
            authentication_details_source: None,
            authentication_entry_point: None,
            custom_login_page: false,
            login_page: "/login".into(),
            login_processing_url: None,
            failure_handler: None,
            permit_all: false,
            failure_url: None,

            inner: Default::default(),
        };

        configurer.set_login_page("/login");

        configurer
    }
}

pub trait BaseAuthenticationFilterConfigurerExt {
    fn login_processing_url(&mut self, login_processing_url: &str);

    fn login_page(&mut self, login_page: &str);

    fn create_login_processing_url_matcher(
        &self,
        login_processing_url: &str,
    ) -> Arc<dyn RequestMatcher> {
        Arc::new(AntPathRequestMatcher::from((
            HttpMethod::Post,
            login_processing_url,
        )))
    }
}
