use std::sync::Arc;

use next_web_core::{
    anys::any_value::AnyValue,
    async_trait,
    traits::{
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
        required::Required,
    },
    util::http_method::HttpMethod,
};
use tracing::{debug, trace};

use crate::{
    core::{
        authc::{
            authentication_error::AuthenticationError, authentication_token::AuthenticationToken,
        },
        subject::Subject,
        util::{object::Object, web::WebUtils},
    },
    web::filter::{
        access_control_filter::{AccessControlFilter, AccessControlFilterExt},
        advice_filter::AdviceFilterExt,
        authc::authenticating_filter::{AuthenticatingFilter, AuthenticatingFilterExt},
        once_per_request_filter::OncePerRequestFilter,
    },
};

#[derive(Clone)]
pub struct FormAuthenticationFilter {
    username_param: Box<str>,
    password_param: Box<str>,
    remember_me_param: Box<str>,
    failure_key_attribute: Box<str>,

    authenticating_filter: AuthenticatingFilter,
}

impl FormAuthenticationFilter {
    pub const DEFAULT_ERROR_KEY_ATTRIBUTE_NAME: &str = "nextLoginFailure";

    pub const DEFAULT_USERNAME_PARAM: &str = "username";

    pub const DEFAULT_PASSWORD_PARAM: &str = "password";

    pub const DEFAULT_REMEMBER_ME_PARAM: &str = "rememberMe";

    pub fn set_login_url(&mut self, login_url: &str) {
        let previous = self.authenticating_filter.get_login_url().to_string();
        if !previous.is_empty() {
            self.authenticating_filter
                .access_control_filter
                .path_matching_filter
                .applied_paths
                .shift_remove(&previous);
        }

        self.authenticating_filter
            .access_control_filter
            .set_login_url(login_url);

        trace!("Adding login url to applied paths.");

        let next = self.authenticating_filter.get_login_url().to_string();
        self.authenticating_filter
            .access_control_filter
            .path_matching_filter
            .applied_paths
            .insert(next, Object::Null);
    }

    pub fn get_user_name_param(&self) -> &str {
        &self.username_param
    }

    pub fn get_password_param(&self) -> &str {
        &self.password_param
    }

    pub fn get_remember_me_param(&self) -> &str {
        &self.remember_me_param
    }

    pub fn get_failure_key_attribute(&self) -> &str {
        &self.failure_key_attribute
    }

    pub fn set_username_param<T: Into<Box<str>>>(&mut self, username_param: T) {
        self.username_param = username_param.into();
    }

    pub fn set_password_param<T: Into<Box<str>>>(&mut self, password_param: T) {
        self.password_param = password_param.into();
    }

    pub fn set_remember_me_param<T: Into<Box<str>>>(&mut self, remember_me_param: T) {
        self.remember_me_param = remember_me_param.into();
    }

    pub fn set_failure_key_attribute<T: Into<Box<str>>>(&mut self, failure_key_attribute: T) {
        self.failure_key_attribute = failure_key_attribute.into();
    }
}

impl FormAuthenticationFilter {
    fn is_login_submission(&self, request: &dyn HttpRequest, _response: &dyn HttpResponse) -> bool {
        request.method() == HttpMethod::Post
    }

    fn get_username<'a>(&self, request: &'a dyn HttpRequest) -> Option<&'a str> {
        WebUtils::get_clean_param(request, self.get_user_name_param())
    }

    fn get_password<'a>(&self, request: &'a dyn HttpRequest) -> Option<&'a str> {
        WebUtils::get_clean_param(request, self.get_password_param())
    }

    fn set_failure_attribute(&self, request: &mut dyn HttpRequest, ae: &AuthenticationError) {
        let type_name = std::any::type_name::<AuthenticationError>().to_string();
        request.set_attribute(
            self.get_failure_key_attribute(),
            AnyValue::String(type_name),
        );
    }
}

#[async_trait]
impl AdviceFilterExt for FormAuthenticationFilter {}

impl Required<OncePerRequestFilter> for FormAuthenticationFilter {
    fn get_object(&self) -> &OncePerRequestFilter {
        todo!()
    }

    fn get_mut_object(&mut self) -> &mut OncePerRequestFilter {
        todo!()
    }
}

impl Named for FormAuthenticationFilter {
    fn name(&self) -> &str {
        "FormAuthenticationFilter"
    }
}

#[async_trait]
impl AccessControlFilterExt for FormAuthenticationFilter {
    async fn on_access_denied(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _mapped_value: Option<Object>,
    ) -> bool {
        if self.is_login_request(request, response) {
            if self.is_login_submission(request, response) {
                trace!("Login submission detected.  Attempting to execute login.");

                return self
                    .authenticating_filter
                    .execute_login(request, response, self)
                    .await
                    .unwrap_or_default();
            } else {
                trace!("Login page view.");
                return true;
            }
        } else {
            trace!(
                "Attempting to access a path which requires authentication.  
            Forwarding to the Authentication url [{}]",
                self.authenticating_filter.get_login_url()
            );

            self.authenticating_filter
                .access_control_filter
                .save_request_and_redirect_to_login(request, response);

            return false;
        }
    }
}

#[async_trait]
impl AuthenticatingFilterExt for FormAuthenticationFilter {
    async fn create_token(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Arc<dyn AuthenticationToken> {
        let username = self.get_username(request).unwrap_or("");
        let password = self.get_password(request).unwrap_or("");

        self.authenticating_filter
            .create_token(username, password, request, response, self)
    }

    fn is_remember_me(&self, request: &dyn HttpRequest) -> bool {
        WebUtils::is_true(request, self.get_remember_me_param())
    }

    async fn on_login_success(
        &self,
        _token: &dyn AuthenticationToken,
        _subject: &mut dyn Subject,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<bool, AuthenticationError> {
        self.authenticating_filter
            .issue_success_redirect(request, response);
        // we handled the success redirect directly, prevent the chain from continuing:
        Ok(false)
    }

    async fn on_login_failure(
        &self,
        _token: &dyn AuthenticationToken,
        error: &AuthenticationError,
        request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
    ) -> bool {
        debug!("Authentication error: {:?}", error);

        self.set_failure_attribute(request, error);
        // login failed, let request continue back to the login page:
        true
    }
}

impl Default for FormAuthenticationFilter {
    fn default() -> Self {
        let mut filter = Self {
            username_param: Self::DEFAULT_USERNAME_PARAM.into(),
            password_param: Self::DEFAULT_PASSWORD_PARAM.into(),
            remember_me_param: Self::DEFAULT_REMEMBER_ME_PARAM.into(),
            failure_key_attribute: Self::DEFAULT_ERROR_KEY_ATTRIBUTE_NAME.into(),
            authenticating_filter: Default::default(),
        };

        filter.set_login_url(AccessControlFilter::<()>::DEFAULT_LOGIN_URL);

        filter
    }
}
