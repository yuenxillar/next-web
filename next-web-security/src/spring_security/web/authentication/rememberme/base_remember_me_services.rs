use std::{error::Error, ops::Deref, sync::Arc};

use base64::{engine::general_purpose::STANDARD, Engine};
use next_web_context::{support::MessageSourceAccessor, MessageSource};
use next_web_core::{
    async_trait,
    http::Cookie,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use tracing::debug;

use crate::{
    authentication::{AccountStatusUserDetailsChecker, RememberMeAuthenticationToken},
    authorization::AuthenticationDetailsSource,
    core::{
        authority::mapping::GrantedAuthoritiesMapper,
        userdetails::{UserDetails, UserDetailsChecker, UserDetailsService},
        Authentication, NextSecurityMessageSource,
    },
    web::authentication::{
        logout::LogoutHandler, remember_me_services::RememberMeServices, AuthPrincipal,
        WebAuthenticationDetailsSource,
    },
};

/// Base implementation of `RememberMeServices`.
/// Subclasses should override `auto_login` with actual token processing logic.
#[derive(Clone)]
pub struct BaseRememberMeServices {
    messages: MessageSourceAccessor,
    user_details_service: Arc<dyn UserDetailsService>,
    user_details_checker: Arc<dyn UserDetailsChecker>,
    authentication_details_source: Arc<dyn AuthenticationDetailsSource>,
    cookie_name: String,
    cookie_domain: Option<String>,
    parameter: String,
    always_remember: bool,
    key: String,
    token_validity_seconds: u32,
    use_secure_cookie: Option<bool>,
    authorities_mapper: Option<Arc<dyn GrantedAuthoritiesMapper>>,
    cookie_customizer: Option<Arc<dyn Fn(&mut Cookie) + Send + Sync>>,
}

impl BaseRememberMeServices {
    pub const NEXT_SECURITY_REMEMBER_ME_COOKIE_KEY: &str = "remember-me";
    pub const DEFAULT_PARAMETER: &str = "remember-me";
    pub const TWO_WEEKS_S: u32 = 1209600;
    const DELIMITER: &str = ":";

    pub fn new(key: impl Into<String>, user_details_service: Arc<dyn UserDetailsService>) -> Self {
        let key = key.into();
        assert!(!key.is_empty(), "key cannot be empty ");

        Self {
            user_details_service,
            key,
            messages: NextSecurityMessageSource::get_accessor(),
            user_details_checker: Arc::new(AccountStatusUserDetailsChecker::default()),
            authentication_details_source: Arc::new(WebAuthenticationDetailsSource::default()),
            cookie_name: Self::NEXT_SECURITY_REMEMBER_ME_COOKIE_KEY.to_string(),
            cookie_domain: None,
            parameter: Self::DEFAULT_PARAMETER.to_string(),
            always_remember: false,
            token_validity_seconds: Self::TWO_WEEKS_S,
            use_secure_cookie: None,
            authorities_mapper: None,
            cookie_customizer: None,
        }
    }

    pub fn after_properties_set(&mut self) {
        assert!(!self.key.is_empty(), "key cannot be empty ");
    }

    /// Locates the Spring Security remember me cookie in the request and returns its
    /// value. The cookie is searched for by name and also by matching the context path to
    /// the cookie path.
    fn extract_remember_me_cookie(&self, request: &dyn HttpRequest) -> Option<String> {
        let cookies = match request.cookies() {
            Some(cookies) => cookies,
            None => return None,
        };
        if cookies.is_empty() {
            return None;
        }

        for cookie in cookies {
            if self.cookie_name == cookie.name() {
                return Some(cookie.value().to_string());
            }
        }

        None
    }

    /// Creates the final `Authentication` object returned from the
    /// `auto_login` method.
    ///
    /// By default it will create a `RememberMeAuthenticationToken` instance.
    fn create_successful_authentication(
        &self,
        request: &dyn HttpRequest,
        user: Arc<dyn UserDetails>,
    ) -> Arc<dyn Authentication> {
        let authorities = user.authorities().to_vec();
        let mut auth = RememberMeAuthenticationToken::new(
            &self.key,
            user as AuthPrincipal,
            self.authorities_mapper
                .as_ref()
                .map(|gam| gam.map_authorities(&authorities).to_vec()),
        );
        auth.set_details(Some(
            self.authentication_details_source.build_details(request),
        ));

        Arc::new(auth)
    }

    /// Decodes the cookie and splits it into a set of token strings using the ":"
    /// delimiter.
    fn decode_cookie(&self, cookie_value: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let mut padded_cookie_value = cookie_value.to_string();
        for _ in 0..(padded_cookie_value.len() % 4) {
            padded_cookie_value.push('=');
        }

        let cookie_as_plain_text =
            String::from_utf8(STANDARD.decode(&padded_cookie_value).map_err(|e| {
                // Create a proper error type, not just a String
                Box::<dyn Error>::from(format!(
                    "Cookie token was not Base64 encoded; value was '{}': {}",
                    padded_cookie_value, e
                ))
            })?)
            .map_err(|e| Box::<dyn Error>::from(format!("Invalid UTF-8 in cookie: {:?}", e)))?;

        let mut tokens: Vec<String> = cookie_as_plain_text
            .split(Self::DELIMITER)
            .map(ToString::to_string)
            .collect();

        for token in &mut tokens {
            *token = urlencoding::decode(token)
                .map_err(|e| format!("URL decode error: {:?}", e))?
                .into_owned();
        }

        Ok(tokens)
    }

    /// Inverse operation of decode_cookie.
    fn encode_cookie(&self, cookie_tokens: &[String]) -> String {
        let encoded_parts: Vec<String> = cookie_tokens
            .iter()
            .map(|token| urlencoding::encode(token).into_owned())
            .collect();
        let value = encoded_parts.join(Self::DELIMITER);
        let encoded = STANDARD.encode(value.as_bytes());

        // Strip trailing '=' characters
        encoded.trim_end_matches('=').to_string()
    }

    /// Allows customization of whether a remember-me login has been requested. The default
    /// is to return true if `always_remember` is set or the configured parameter
    /// name has been included in the request and is set to the value "true".
    fn remember_me_requested(&self, request: &dyn HttpRequest, parameter: &str) -> bool {
        if self.always_remember {
            return true;
        }
        if let Some(param_value) = request.parameter(parameter) {
            let lower = param_value.to_lowercase();
            if lower == "true" || lower == "on" || lower == "yes" || lower == "1" {
                return true;
            }
        }
        debug!(
            "Did not send remember-me cookie (principal did not set parameter '{}')",
            parameter
        );
        false
    }

    /// Sets a "cancel cookie" (with maxAge = 0) on the response to disable persistent  logins.
    fn cancel_cookie(&self, request: &dyn HttpRequest, response: &mut dyn HttpResponse) {
        debug!("Cancelling cookie");
        let mut cookie = Cookie::new(&self.cookie_name, None);
        cookie.set_max_age_secs(0);
        cookie.set_path(self.get_cookie_path(request));
        if let Some(domain) = self.cookie_domain.as_ref() {
            cookie.set_domain(domain);
        }
        cookie.set_secure(
            self.use_secure_cookie
                .unwrap_or_else(|| request.is_secure()),
        );
        response.add_cookie(cookie);
    }

    /// Sets the cookie on the response.
    ///
    /// By default a secure cookie will be used if the connection is secure. You can set
    /// the `use_secure_cookie` property to `false` to override this. If you set
    /// it to `true`, the cookie will always be flagged as secure. By default the
    /// cookie will be marked as HttpOnly.
    pub(crate) fn set_cookie(
        &self,
        tokens: &[String],
        max_age: u32,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) {
        let cookie_value = self.encode_cookie(tokens);
        let mut cookie = Cookie::new(&self.cookie_name, Some(cookie_value));
        cookie.set_max_age_secs(max_age);
        cookie.set_path(self.get_cookie_path(request));
        if let Some(domain) = self.cookie_domain.as_ref() {
            cookie.set_domain(domain);
        }
        cookie.set_secure(
            self.use_secure_cookie
                .unwrap_or_else(|| request.is_secure()),
        );
        cookie.set_http_only(true);

        self.cookie_customizer.as_ref().map(|f| f(&mut cookie));

        response.add_cookie(cookie);
    }

    fn get_cookie_path<'a>(&self, request: &'a dyn HttpRequest) -> &'a str {
        request
            .context_path()
            .filter(|s| !s.is_empty())
            .unwrap_or("/")
    }

    pub fn set_cookie_name(&mut self, cookie_name: impl Into<String>) {
        let cookie_name = cookie_name.into();
        assert!(!cookie_name.is_empty(), "Cookie name cannot be empty");

        self.cookie_name = cookie_name;
    }

    pub fn set_cookie_domain(&mut self, cookie_domain: impl Into<String>) {
        let cookie_domain = cookie_domain.into();
        assert!(!cookie_domain.is_empty(), "Cookie domain cannot be empty");
        self.cookie_domain = Some(cookie_domain);
    }

    pub fn get_cookie_name(&self) -> &str {
        &self.cookie_name
    }

    pub fn set_always_remember(&mut self, always_remember: bool) {
        self.always_remember = always_remember;
    }

    /// Sets the name of the parameter which should be checked for to see if a remember-me
    /// has been requested during a login request. This should be the same name you assign
    /// to the checkbox in your login form.
    pub fn set_parameter(&mut self, parameter: impl Into<String>) {
        let parameter = parameter.into();
        assert!(
            !parameter.trim().is_empty(),
            "Parameter name cannot be empty"
        );
        self.parameter = parameter;
    }

    pub fn get_parameter(&self) -> &str {
        &self.parameter
    }

    pub fn get_user_details_service(&self) -> &dyn UserDetailsService {
        self.user_details_service.as_ref()
    }

    pub fn get_key(&self) -> &str {
        &self.key
    }

    pub fn set_token_validity_seconds(&mut self, token_validity_seconds: u32) {
        self.token_validity_seconds = token_validity_seconds;
    }

    pub fn get_token_validity_seconds(&self) -> u32 {
        self.token_validity_seconds
    }

    /// Whether the cookie should be flagged as secure or not. Secure cookies can only be
    /// sent over an HTTPS connection and thus cannot be accidentally submitted over HTTP
    /// where they could be intercepted.
    ///
    /// By default the cookie will be secure if the request is secure. If you only want to
    /// use remember-me over HTTPS (recommended) you should set this property to
    /// `true`.
    pub fn set_use_secure_cookie(&mut self, use_secure_cookie: bool) {
        self.use_secure_cookie = Some(use_secure_cookie);
    }

    pub fn get_authentication_details_source(&self) -> &dyn AuthenticationDetailsSource {
        self.authentication_details_source.as_ref()
    }

    pub fn set_authentication_details_source(
        &mut self,
        authentication_details_source: Arc<dyn AuthenticationDetailsSource>,
    ) {
        self.authentication_details_source = authentication_details_source;
    }

    /// Sets the strategy to be used to validate the `UserDetails` object obtained
    /// for the user when processing a remember-me cookie to automatically log in a user.
    pub fn set_user_details_checker(&mut self, user_details_checker: Arc<dyn UserDetailsChecker>) {
        self.user_details_checker = user_details_checker;
    }

    pub fn set_authorities_mapper(
        &mut self,
        authorities_mapper: Arc<dyn GrantedAuthoritiesMapper>,
    ) {
        self.authorities_mapper = Some(authorities_mapper);
    }

    /// Sets the `MessageSource` for i18n messages.
    pub fn set_message_source(&mut self, message_source: Arc<dyn MessageSource>) {
        self.messages = MessageSourceAccessor::new(message_source);
    }

    /// Sets the callback, allowing customization of cookie.
    pub fn set_cookie_customizer<F>(&mut self, cookie_customizer: F)
    where
        F: Fn(&mut Cookie) + 'static,
        F: Send + Sync,
    {
        self.cookie_customizer = Some(Arc::new(cookie_customizer));
    }
}

#[async_trait]
impl<T> RememberMeServices for T
where
    T: Deref<Target = BaseRememberMeServices>,
    T: BaseRememberMeServicesExt,
    T: 'static,
{
    /// Template implementation which locates the Spring Security cookie, decodes it into a
    /// delimited array of tokens and submits it to subclasses for processing via the
    /// `process_auto_login_cookie` method.
    ///
    /// The returned username is then used to load the UserDetails object for the user,
    /// which in turn is used to create a valid authentication token.
    async fn auto_login(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Option<Arc<dyn Authentication>> {
        let remember_me_cookie = match self.extract_remember_me_cookie(request) {
            Some(cookie) => cookie,
            None => {
                return None;
            }
        };

        debug!("Remember-me cookie detected");
        if remember_me_cookie.is_empty() {
            debug!("Cookie was empty");
            self.cancel_cookie(request, response);
            return None;
        }

        let cookie_tokens = match self.decode_cookie(&remember_me_cookie) {
            Ok(tokens) => tokens,
            Err(e) => {
                debug!("{}", e);
                self.cancel_cookie(request, response);
                return None;
            }
        };

        let user = match self
            .process_auto_login_cookie(&cookie_tokens, request, response)
            .await
        {
            Ok(u) => u,
            Err(e) => {
                debug!("{}", e);
                self.cancel_cookie(request, response);
                return None;
            }
        };

        if let Err(e) = self.user_details_checker.check(user.as_ref()) {
            debug!("{}", e);
            self.cancel_cookie(request, response);
            return None;
        }

        debug!("Remember-me cookie accepted");
        Some(self.create_successful_authentication(request, user))
    }

    fn login_fail(&self, request: &mut dyn HttpRequest, response: &mut dyn HttpResponse) {
        debug!("Interactive login attempt was unsuccessful.");

        self.cancel_cookie(request, response);
        self.on_login_fail(request, response);
    }

    /// Examines the incoming request and checks for the presence of the configured
    /// "remember me" parameter. If it's present, or if `always_remember` is set to
    /// true, calls `on_login_success`.
    fn login_success(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        successful_authentication: &dyn Authentication,
    ) {
        if !self.remember_me_requested(request, &self.parameter) {
            debug!("Remember-me login not requested.");
            return;
        }
        self.on_login_success(request, response, successful_authentication);
    }
}

#[async_trait]
impl LogoutHandler for BaseRememberMeServices {
    /// Implementation of `LogoutHandler`. Default behaviour is to call
    /// `cancel_cookie()`.
    async fn logout(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: Option<&Arc<dyn Authentication>>,
    ) {
        match authentication {
            Some(s) => debug!("Logout of user {}", s.name()),
            None => debug!("Logout of user Unknown"),
        };

        self.cancel_cookie(request, response);
    }
}

#[async_trait]
pub trait BaseRememberMeServicesExt
where
    Self: Send + Sync,
{
    /// Called from login_success when a remember-me login has been requested. Typically
    /// implemented by subclasses to set a remember-me cookie and potentially store a
    /// record of it if the implementation requires this.
    async fn on_login_success(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        successful_authentication: &dyn Authentication,
    );

    #[allow(unused_variables)]
    fn on_login_fail(&self, request: &mut dyn HttpRequest, response: &mut dyn HttpResponse) {}

    /// Called from auto_login to process the submitted persistent login cookie. Subclasses
    /// should validate the cookie and perform any additional management required.
    async fn process_auto_login_cookie(
        &self,
        cookie_tokens: &[String],
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<Arc<dyn UserDetails>, Box<dyn Error>>;
}
