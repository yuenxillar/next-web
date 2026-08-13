use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use next_web_core::error::BoxError;
use next_web_core::AnyObject;
use next_web_core::{
    async_trait,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
    util::HttpMethod,
};

use crate::authentication::EMPTY_STRING;
use crate::authorization::AuthenticationManager;
use crate::core::Authentication;
use crate::web::util::matcher::{PathPatternRequestMatcher, RequestMatcher};
use crate::{
    core::UsernamePasswordAuthenticationToken,
    web::authentication::BaseAuthenticationProcessingFilter,
};

#[derive(Clone)]
pub struct UsernamePasswordAuthenticationFilter {
    username_parameter: Box<str>,
    password_parameter: Box<str>,
    post_only: bool,

    inner: BaseAuthenticationProcessingFilter,
}

impl UsernamePasswordAuthenticationFilter {
    pub const NEXT_SECURITY_FORM_USERNAME_KEY: &'static str = "username";
    pub const NEXT_SECURITY_FORM_PASSWORD_KEY: &'static str = "password";

    pub fn new(authentication_manager: Arc<dyn AuthenticationManager>) -> Self {
        Self {
            username_parameter: Self::NEXT_SECURITY_FORM_USERNAME_KEY.into(),
            password_parameter: Self::NEXT_SECURITY_FORM_PASSWORD_KEY.into(),
            post_only: true,
            inner: BaseAuthenticationProcessingFilter::with_matcher_and_manager(
                Self::default_path_request_matcher(),
                authentication_manager,
            ),
        }
    }

    fn attempt_authentication(
        &self,
        request: &dyn HttpRequest,
        _response: &mut dyn HttpResponse,
    ) -> Result<Option<Arc<dyn Authentication>>, BoxError> {
        if self.post_only && request.method() != HttpMethod::Post {
            return Err(format!(
                "Authentication method not supported: {:?}",
                request.method()
            )
            .into());
        }

        let username = self
            .obtain_username(request)
            .map(str::trim)
            .map(ToString::to_string)
            .map(|s| Arc::new(s) as AnyObject)
            .unwrap_or_else(|| EMPTY_STRING.to_owned());
        let password = self
            .obtain_password(request)
            .map(ToString::to_string)
            .map(|s| Arc::new(s) as AnyObject)
            .unwrap_or_else(|| EMPTY_STRING.to_owned());

        let mut auth_request =
            UsernamePasswordAuthenticationToken::unauthenticated(Some(username), Some(password));

        // Allow subclasses to set the "details" property
        self.set_details(request, &mut auth_request);
        self.get_authentication_manager()
            .map(|manager| manager.authenticate(&auth_request).map_err(Into::into))
            .transpose()
    }

    /// Enables subclasses to override the composition of the password, such as by including additional values and a separator.
    /// This might be used for example if a postcode/zipcode was required in addition to the password. A delimiter such as a pipe (|)
    /// should be used to separate the password and extended value(s). The AuthenticationDao will
    /// need to generate the expected password in a corresponding manner.
    pub fn obtain_password<'a>(&self, request: &'a dyn HttpRequest) -> Option<&'a str> {
        request.parameter(self.password_parameter.as_ref())
    }

    /// Enables subclasses to override the composition of the username, such as by including additional values and a separator.
    pub fn obtain_username<'a>(&self, request: &'a dyn HttpRequest) -> Option<&'a str> {
        request.parameter(self.username_parameter.as_ref())
    }

    /// Provided so that subclasses may configure what is put into the authentication request's details property.
    pub fn set_details(
        &self,
        request: &dyn HttpRequest,
        auth_request: &mut UsernamePasswordAuthenticationToken,
    ) {
        auth_request.set_details(Some(
            self.inner
                .authentication_details_source
                .build_details(request),
        ));
    }

    /// Sets the parameter name which will be used to obtain the username from the login request.
    pub fn set_username_parameter(&mut self, username_parameter: &str) {
        assert!(
            !username_parameter.trim().is_empty(),
            "username_parameter cannot be empty"
        );
        self.username_parameter = username_parameter.into();
    }

    /// Sets the parameter name which will be used to obtain the password from the login request.
    pub fn set_password_parameter(&mut self, password_parameter: &str) {
        assert!(
            !password_parameter.trim().is_empty(),
            "password_parameter cannot be empty"
        );
        self.password_parameter = password_parameter.into();
    }

    /// Defines whether only HTTP POST requests will be allowed by this filter. If set to true,
    /// and an authentication request is received which is not a POST request, an exception will be raised immediately and authentication will not be attempted.
    /// The unsuccessfulAuthentication() method will be called as if handling a failed authentication.
    /// Defaults to true but may be overridden by subclasses.
    pub fn set_post_only(&mut self, post_only: bool) {
        self.post_only = post_only;
    }

    pub fn get_username_parameter(&self) -> &str {
        &self.username_parameter
    }

    pub fn get_password_parameter(&self) -> &str {
        &self.password_parameter
    }

    fn default_path_request_matcher() -> Arc<dyn RequestMatcher> {
        Arc::new(
            PathPatternRequestMatcher::with_defaults().matcher(Some(HttpMethod::Post), "/login"),
        )
    }
}

impl Deref for UsernamePasswordAuthenticationFilter {
    type Target = BaseAuthenticationProcessingFilter;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for UsernamePasswordAuthenticationFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[async_trait]
impl HttpFilter for UsernamePasswordAuthenticationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        self.inner.do_filter(request, response, filter_chain).await
    }
}

impl Named for UsernamePasswordAuthenticationFilter {
    fn name(&self) -> &str {
        "UsernamePasswordAuthenticationFilter"
    }
}

impl Default for UsernamePasswordAuthenticationFilter {
    fn default() -> Self {
        Self {
            username_parameter: Self::NEXT_SECURITY_FORM_USERNAME_KEY.into(),
            password_parameter: Self::NEXT_SECURITY_FORM_PASSWORD_KEY.into(),
            post_only: true,

            inner: BaseAuthenticationProcessingFilter::with_request_matcher(
                Self::default_path_request_matcher(),
            ),
        }
    }
}
