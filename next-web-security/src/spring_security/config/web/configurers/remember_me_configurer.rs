use std::{
    any::Any,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::traits::any_clone::AnyClone;

use crate::{
    authentication::remember_me_authentication_provider::RememberMeAuthenticationProvider,
    authorization::AuthenticationManager,
    config::{
        security_configurer::SecurityConfigurer,
        web::{
            configurers::{
                base_http_configurer::BaseHttpConfigurer, LogoutConfigurer,
                SecurityContextConfigurer,
            },
            http_security_builder::HttpSecurityBuilder,
        },
    },
    core::userdetails::UserDetailsService,
    web::{
        authentication::{
            logout::LogoutHandler,
            rememberme::{
                PersistentTokenBasedRememberMeServices, PersistentTokenRepository,
                TokenBasedRememberMeServices,
            },
            session::SessionAuthenticationStrategy,
            ui::DefaultLoginPageGeneratingFilter,
            AuthenticationSuccessHandler, RememberMeAuthenticationFilter, RememberMeServices,
        },
        default_security_filter_chain::DefaultSecurityFilterChain,
    },
};

/// Configures Remember Me authentication. This typically involves the user checking a box when they
/// enter their username and password that states to "Remember Me".
#[derive(Clone)]
pub struct RememberMeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    authentication_success_handler: Option<Arc<dyn AuthenticationSuccessHandler>>,
    key: Option<String>,
    remember_me_services: Option<Arc<dyn RememberMeServices>>,
    logout_handler: Option<Arc<dyn LogoutHandler>>,
    remember_me_parameter: String,
    remember_me_cookie_name: String,
    remember_me_cookie_domain: Option<String>,
    token_repository: Option<Arc<dyn PersistentTokenRepository>>,
    user_details_service: Option<Arc<dyn UserDetailsService>>,
    token_validity_seconds: Option<i32>,
    use_secure_cookie: Option<bool>,
    always_remember: Option<bool>,

    inner: BaseHttpConfigurer<Self, H>,
}

impl<H> RememberMeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    const DEFAULT_REMEMBER_ME_NAME: &'static str = "remember-me";

    /// Allows specifying how long (in seconds) a token is valid for
    ///
    /// # See also
    /// [`AbstractRememberMeServices::set_token_validity_seconds`]
    pub fn token_validity_seconds(&mut self, token_validity_seconds: i32) -> &mut Self {
        self.token_validity_seconds = Some(token_validity_seconds);
        self
    }

    /// Whether the cookie should be flagged as secure or not.
    ///
    /// Secure cookies can only be sent over an HTTPS connection and thus cannot be
    /// accidentally submitted over HTTP where they could be intercepted.
    ///
    /// By default the cookie will be secure if the request is secure. If you only want
    /// to use remember-me over HTTPS (recommended) you should set this property to
    /// `true`.
    ///
    /// # See also
    /// [`AbstractRememberMeServices::set_use_secure_cookie`]
    pub fn use_secure_cookie(&mut self, use_secure_cookie: bool) -> &mut Self {
        self.use_secure_cookie = Some(use_secure_cookie);
        self
    }

    /// Specifies the `UserDetailsService` used to look up the `UserDetails` when a
    /// remember me token is valid.
    ///
    /// When using a `SecurityFilterChain` bean, the default is to look for a
    /// `UserDetailsService` bean. Alternatively, one can populate
    /// [`Self::remember_me_services`].
    ///
    /// # See also
    /// [`AbstractRememberMeServices`]
    pub fn user_details_service(
        &mut self,
        user_details_service: Arc<dyn UserDetailsService>,
    ) -> &mut Self {
        self.user_details_service = Some(user_details_service);
        self
    }

    /// Specifies the `PersistentTokenRepository` to use.
    ///
    /// The default is to use `TokenBasedRememberMeServices` instead.
    pub fn token_repository(
        &mut self,
        token_repository: Arc<dyn PersistentTokenRepository>,
    ) -> &mut Self {
        self.token_repository = Some(token_repository);
        self
    }

    /// Sets the key to identify tokens created for remember me authentication.
    ///
    /// Default is a secure randomly generated key. If
    /// [`Self::remember_me_services`] is specified and is of type
    /// `AbstractRememberMeServices`, then the default is the key set in
    /// `AbstractRememberMeServices`.
    pub fn key(&mut self, key: impl Into<String>) -> &mut Self {
        self.key = Some(key.into());
        self
    }

    /// The HTTP parameter used to indicate to remember the user at time of login.
    pub fn remember_me_parameter(&mut self, remember_me_parameter: impl Into<String>) -> &mut Self {
        self.remember_me_parameter = remember_me_parameter.into();
        self
    }

    /// The name of cookie which store the token for remember me authentication.
    ///
    /// Defaults to 'remember-me'.
    pub fn remember_me_cookie_name(
        &mut self,
        remember_me_cookie_name: impl Into<String>,
    ) -> &mut Self {
        self.remember_me_cookie_name = remember_me_cookie_name.into();
        self
    }

    /// The domain name within which the remember me cookie is visible.
    pub fn remember_me_cookie_domain(
        &mut self,
        remember_me_cookie_domain: impl Into<String>,
    ) -> &mut Self {
        self.remember_me_cookie_domain = Some(remember_me_cookie_domain.into());
        self
    }

    /// Allows control over the destination a remembered user is sent to when they are
    /// successfully authenticated.
    ///
    /// By default, the filter will just allow the current request to proceed, but if an
    /// `AuthenticationSuccessHandler` is set, it will be invoked and the `do_filter()`
    /// method will return immediately, thus allowing the application to redirect the
    /// user to a specific URL, regardless of what the original request was for.
    ///
    /// # See also
    /// [`RememberMeAuthenticationFilter::set_authentication_success_handler`]
    pub fn authentication_success_handler(
        mut self,
        authentication_success_handler: Arc<dyn AuthenticationSuccessHandler>,
    ) -> Self {
        self.authentication_success_handler = Some(authentication_success_handler);
        self
    }

    /// Specify the `RememberMeServices` to use.
    ///
    /// # See also
    /// [`RememberMeServices`]
    pub fn remember_me_services(
        &mut self,
        remember_me_services: Arc<dyn RememberMeServices>,
    ) -> &mut Self {
        self.remember_me_services = Some(remember_me_services);
        self
    }

    /// Whether the cookie should always be created even if the remember-me parameter is
    /// not set.
    ///
    /// By default this will be set to `false`.
    ///
    /// # See also
    /// [`AbstractRememberMeServices::set_always_remember`]
    pub fn always_remember(&mut self, always_remember: bool) -> &mut Self {
        self.always_remember = Some(always_remember);
        self
    }

    /// Validate remember_me_services and remember_me_cookie_name have not been set at
    /// the same time.
    fn validate_input(&self) {
        if self.remember_me_services.is_some()
            && self.remember_me_cookie_name != Self::DEFAULT_REMEMBER_ME_NAME
        {
            panic!("Can not set remember_me_cookie_name and custom remember_me_services.");
        }
    }

    /// Returns the HTTP parameter used to indicate to remember the user at time of login.
    fn get_remember_me_parameter(&self) -> &str {
        &self.remember_me_parameter
    }

    /// If available, initializes the DefaultLoginPageGeneratingFilter shared object.
    fn init_default_login_filter(&self, http: &mut H) {
        if let Some(login_page_generating_filter) =
            http.shared_object_mut::<DefaultLoginPageGeneratingFilter>()
        {
            login_page_generating_filter
                .set_remember_me_parameter(self.get_remember_me_parameter());
        }
    }

    /// Gets the `RememberMeServices` or creates the `RememberMeServices`.
    fn get_remember_me_services(&mut self, http: &mut H, key: &str) -> Arc<dyn RememberMeServices> {
        if let Some(remember_me_services) = self.remember_me_services.as_ref() {
            // Check if it's also a LogoutHandler
            // In Rust, you'd typically check via downcast or trait object casting
            // For simplicity, assume we handle this elsewhere
            if self.logout_handler.is_none() {
                // self.logout_handler = Some(remember_me_services.clone_as_logout_handler());
            }
            return remember_me_services.clone();
        }

        if self.token_repository.is_some() {
            let token_remember_me_services =
                Arc::new(self.create_persistent_remember_me_services(http, key));
            self.logout_handler = Some(token_remember_me_services.clone());
            self.remember_me_services = Some(token_remember_me_services.clone());
            return token_remember_me_services;
        } else {
            let token_remember_me_services =
                Arc::new(self.create_token_based_remember_me_services(http, key));

            self.logout_handler = Some(token_remember_me_services.clone());
            self.remember_me_services = Some(token_remember_me_services.clone());
            return token_remember_me_services;
        }
    }

    /// Creates `TokenBasedRememberMeServices`
    fn create_token_based_remember_me_services(
        &self,
        http: &H,
        key: &str,
    ) -> TokenBasedRememberMeServices {
        let user_details_service = self.get_user_details_service(http);
        let mut remember_me_services = TokenBasedRememberMeServices::new(key, user_details_service);

        remember_me_services.set_parameter(&self.remember_me_parameter);
        remember_me_services.set_cookie_name(&self.remember_me_cookie_name);
        if let Some(remember_me_cookie_domain) = self.remember_me_cookie_domain.as_deref() {
            remember_me_services.set_cookie_domain(remember_me_cookie_domain);
        }
        if let Some(token_validity_seconds) = self.token_validity_seconds {
            remember_me_services.set_token_validity_seconds(token_validity_seconds);
        }
        if let Some(use_secure_cookie) = self.use_secure_cookie {
            remember_me_services.set_use_secure_cookie(use_secure_cookie);
        }
        if let Some(always_remember) = self.always_remember {
            remember_me_services.set_always_remember(always_remember);
        }
        remember_me_services.after_properties_set();

        remember_me_services
    }

    /// Creates `PersistentTokenBasedRememberMeServices`
    fn create_persistent_remember_me_services(
        &self,
        http: &H,
        key: &str,
    ) -> PersistentTokenBasedRememberMeServices {
        let user_details_service = self.get_user_details_service(http);
        let token_repository = self
            .token_repository
            .as_ref()
            .map(Clone::clone)
            .expect("token_repository must be set for persistent remember me services");

        let mut remember_me_services = PersistentTokenBasedRememberMeServices::new(
            key,
            user_details_service,
            token_repository,
        );
        remember_me_services.set_parameter(&self.remember_me_parameter);
        remember_me_services.set_cookie_name(&self.remember_me_cookie_name);
        if let Some(remember_me_cookie_domain) = self.remember_me_cookie_domain.as_deref() {
            remember_me_services.set_cookie_domain(remember_me_cookie_domain);
        }
        if let Some(token_validity_seconds) = self.token_validity_seconds {
            remember_me_services.set_token_validity_seconds(token_validity_seconds);
        }
        if let Some(use_secure_cookie) = self.use_secure_cookie {
            remember_me_services.set_use_secure_cookie(use_secure_cookie);
        }
        if let Some(always_remember) = self.always_remember {
            remember_me_services.set_always_remember(always_remember);
        }
        remember_me_services.after_properties_set();

        remember_me_services
    }

    /// Gets the `UserDetailsService` to use.
    ///
    /// Either the explicitly configured `UserDetailsService` from
    /// [`Self::user_details_service`], a shared object from `HttpSecurity`, or the
    /// `UserDetailsService` bean.
    fn get_user_details_service(&self, http: &H) -> Arc<dyn UserDetailsService> {
        if let Some(user_details_service) = self.user_details_service.as_ref() {
            return user_details_service.clone();
        }

        let user_details_service =
            self.get_shared_or_singleton::<Arc<dyn UserDetailsService>>(http);
        assert!(
              user_details_service.is_some(),
              "user_details_service cannot be null. Invoke RememberMeConfigurer::user_details_service or see its documentation for alternative approaches."
          );
        user_details_service.unwrap()
    }

    /// Gets the key to use for validating remember me tokens. If a value was passed into key(String),
    /// then that is returned. Alternatively, if a key was specified in the rememberMeServices(RememberMeServices)},
    /// then that is returned. If no key was specified in either of those cases, then a secure random string is generated.
    fn get_key(&self) -> String {
        match self.key.as_deref() {
            Some(s) => s.to_string(),
            None => self
                .remember_me_services
                .as_ref()
                .and_then(|services| {
                    let services = services as &dyn Any;
                    services
                        .downcast_ref::<TokenBasedRememberMeServices>()
                        .map(|s| s.get_key())
                        .or(services
                            .downcast_ref::<PersistentTokenBasedRememberMeServices>()
                            .map(|s| s.get_key()))
                        .map(ToString::to_string)
                })
                .unwrap_or(uuid::Uuid::new_v4().to_string()),
        }
    }

    fn get_shared_or_singleton<T>(&self, http: &H) -> Option<T>
    where
        T: AnyClone,
        T: Clone,
    {
        if let Some(shared) = http.shared_object::<T>().map(Clone::clone) {
            return Some(shared);
        }

        todo!("Application context get singleton")
    }
}

impl<H> Default for RememberMeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            authentication_success_handler: None,
            key: None,
            remember_me_services: None,
            logout_handler: None,
            remember_me_parameter: Self::DEFAULT_REMEMBER_ME_NAME.to_string(),
            remember_me_cookie_name: Self::DEFAULT_REMEMBER_ME_NAME.to_string(),
            remember_me_cookie_domain: None,
            token_repository: None,
            user_details_service: None,
            token_validity_seconds: None,
            use_secure_cookie: None,
            always_remember: None,

            inner: Default::default(),
        }
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for RememberMeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    fn init(&mut self, http: &mut H) {
        self.validate_input();
        let key = self.get_key();
        let remember_me_services = self.get_remember_me_services(http, &key);
        http.set_shared_object::<Arc<dyn RememberMeServices>>(remember_me_services);

        // Register logout handler if LogoutConfigurer is present
        if let Some(logout_configurer) = http.configurer_mut::<LogoutConfigurer<H>>() {
            if let Some(logout_handler) = self.logout_handler.as_ref() {
                logout_configurer.add_logout_handler(logout_handler.to_owned());
            }
        }

        let authentication_provider = Arc::new(RememberMeAuthenticationProvider::new(key));

        // authentication_provider = post_process(authentication_provider);
        http.authentication_provider(authentication_provider);
        self.init_default_login_filter(http);
    }

    fn configure(&mut self, http: &mut H) {
        let authentication_manager = http
            .shared_object::<Arc<dyn AuthenticationManager>>()
            .map(Clone::clone)
            .expect("AuthenticationManager is required");

        let remember_me_services = self
            .remember_me_services
            .take()
            .expect("RememberMeServices must be initialized");

        let mut remember_me_filter =
            RememberMeAuthenticationFilter::new(authentication_manager, remember_me_services);

        if let Some(auth_success_handler) = self.authentication_success_handler.take() {
            remember_me_filter.set_authentication_success_handler(auth_success_handler);
        }

        // Handle SecurityContextConfigurer if present and requires explicit save
        if let Some(security_context_configurer) = http.configurer::<SecurityContextConfigurer<H>>()
        {
            if security_context_configurer.is_require_explicit_save() {
                let security_context_repository =
                    security_context_configurer.get_security_context_repository(http);
                remember_me_filter.set_security_context_repository(security_context_repository);
            }
        }

        remember_me_filter.set_security_context_holder_strategy(
            self.inner.get_security_context_holder_strategy().to_owned(),
        );

        // Set session authentication strategy if available
        if let Some(session_auth_strategy) =
            http.shared_object::<Arc<dyn SessionAuthenticationStrategy>>()
        {
            remember_me_filter
                .set_session_authentication_strategy(session_auth_strategy.to_owned());
        }

        // remember_me_filter = post_process(remember_me_filter);
        http.add_filter(remember_me_filter);
    }
}

impl<H> Deref for RememberMeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<H> DerefMut for RememberMeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
