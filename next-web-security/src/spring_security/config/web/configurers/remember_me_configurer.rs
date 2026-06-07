use std::sync::Arc;

use next_web_core::traits::required::Required;

use crate::{
    authentication::remember_me_authentication_provider::RememberMeAuthenticationProvider,
    authorization::AuthenticationManager,
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{
            configurers::base_http_configurer::BaseHttpConfigurer,
            http_security_builder::HttpSecurityBuilder,
        },
    },
    web::{
        authentication::{
            remember_me_authentication_filter::RememberMeAuthenticationFilter,
            remember_me_services::RememberMeServices,
            rememberme::abstract_remember_me_services::AbstractRememberMeServices,
        },
        default_security_filter_chain::DefaultSecurityFilterChain,
    },
};

/// Configures Remember-Me authentication (persistent cookie-based login).
///
/// The default key is randomly generated. Override with `.key("my-secret")`.
#[derive(Clone)]
pub struct RememberMeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<RememberMeConfigurer<H>, H>>,
{
    key: Option<String>,
    remember_me_services: Option<Arc<dyn RememberMeServices>>,
    token_validity_seconds: Option<i64>,
    remember_me_parameter: String,
    remember_me_cookie_name: String,
    always_remember: bool,
    use_secure_cookie: Option<bool>,

    base_http_configurer: BaseHttpConfigurer<RememberMeConfigurer<H>, H>,
}

impl<H> RememberMeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<RememberMeConfigurer<H>, H>>,
{
    /// Set the secret key for remember-me token hashing.
    pub fn key(mut self, key: &str) -> Self {
        self.key = Some(key.to_string());
        self
    }

    /// Set a custom `RememberMeServices` implementation.
    pub fn remember_me_services(
        mut self,
        services: Arc<dyn RememberMeServices>,
    ) -> Self {
        self.remember_me_services = Some(services);
        self
    }

    /// Set the token validity period in seconds.
    pub fn token_validity_seconds(mut self, seconds: i64) -> Self {
        self.token_validity_seconds = Some(seconds);
        self
    }

    /// Set the HTTP request parameter for the "remember me" checkbox.
    /// Default: `"remember-me"`.
    pub fn remember_me_parameter(mut self, parameter: &str) -> Self {
        self.remember_me_parameter = parameter.to_string();
        self
    }

    /// Set the cookie name for the remember-me token.
    /// Default: `"remember-me"`.
    pub fn remember_me_cookie_name(mut self, cookie_name: &str) -> Self {
        self.remember_me_cookie_name = cookie_name.to_string();
        self
    }

    /// Always remember the user (skip the checkbox).
    pub fn always_remember(mut self, always: bool) -> Self {
        self.always_remember = always;
        self
    }

    /// Require a secure (HTTPS) cookie for the remember-me token.
    pub fn use_secure_cookie(mut self, secure: bool) -> Self {
        self.use_secure_cookie = Some(secure);
        self
    }

    fn get_key(&self) -> String {
        self.key
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
    }

    fn get_remember_me_services(&self) -> Arc<dyn RememberMeServices> {
        self.remember_me_services
            .clone()
            .unwrap_or_else(|| Arc::new(AbstractRememberMeServices {}))
    }
}

impl<H> Default for RememberMeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<RememberMeConfigurer<H>, H>>,
{
    fn default() -> Self {
        Self {
            key: None,
            remember_me_services: None,
            token_validity_seconds: None,
            remember_me_parameter: "remember-me".to_string(),
            remember_me_cookie_name: "remember-me".to_string(),
            always_remember: false,
            use_secure_cookie: None,
            base_http_configurer: Default::default(),
        }
    }
}

impl<H> Required<BaseHttpConfigurer<RememberMeConfigurer<H>, H>>
    for RememberMeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<RememberMeConfigurer<H>, H> {
        &self.base_http_configurer
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut BaseHttpConfigurer<RememberMeConfigurer<H>, H> {
        &mut self.base_http_configurer
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for RememberMeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base_http_configurer.get_object()
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base_http_configurer.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for RememberMeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, http: &mut H) {
        // Register the RememberMeAuthenticationProvider
        let provider =
            RememberMeAuthenticationProvider::new(&self.get_key());
        http.authentication_provider(provider);

        // Set RememberMeServices as a shared object
        let services = self.get_remember_me_services();
        http.set_shared_object("remember_me_services", services);
    }

    fn configure(&mut self, http: &mut H) {
        // Obtain the AuthenticationManager from shared objects
        let auth_manager: Arc<dyn AuthenticationManager> = match http
            .get_shared_object::<Arc<dyn AuthenticationManager>>()
        {
            Some(m) => m.clone(),
            None => panic!(
                "AuthenticationManager is required for RememberMeConfigurer. \
                 Ensure authentication_manager() has been configured."
            ),
        };

        let services = self.get_remember_me_services();
        let filter =
            RememberMeAuthenticationFilter::new(auth_manager, services);
        http.add_filter(filter);
    }
}
