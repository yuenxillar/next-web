use std::borrow::Cow;
use std::sync::Arc;

use next_web_core::{
    Named, Required,
    any::AnyValue,
    async_trait,
    http::{HttpMethod, HttpRequest, HttpResponse},
};
use tracing::{debug, trace};

use crate::{
    core::{
        authc::{AuthenticationError, authentication_token::AuthenticationToken},
        subject::Subject,
        util::{object::Object, web::WebUtils},
    },
    web::filter::{
        access_control_filter::{AccessControlFilter, AccessControlFilterExt},
        advice_filter::AdviceFilterExt,
        authc::{
            authenticating_filter::{AuthenticatingFilter, AuthenticatingFilterExt},
            authentication_filter::AuthenticationFilter,
        },
        once_per_request_filter::OncePerRequestFilter,
        path_matching_filter::{PathMatchingFilter, PathMatchingFilterExt},
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

    pub fn get_login_url(&self) -> &str {
        self.authenticating_filter.get_login_url()
    }

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
        request.method() == HttpMethod::POST
    }

    /// Returns the user name of the submission.
    ///
    /// A form carries its values in the body of the post, not in the query string,
    /// so the parameter is read with the lookup that covers both — which is what a
    /// servlet request does for `getParameter`.
    fn get_username<'a>(&self, request: &'a dyn HttpRequest) -> Option<Cow<'a, str>> {
        request.form_parameter(self.get_user_name_param())
    }

    /// Returns the password of the submission.
    fn get_password<'a>(&self, request: &'a dyn HttpRequest) -> Option<Cow<'a, str>> {
        request.form_parameter(self.get_password_param())
    }

    fn set_failure_attribute(&self, request: &mut dyn HttpRequest, _ae: &AuthenticationError) {
        let type_name = std::any::type_name::<AuthenticationError>().to_string();
        request.set_attribute(
            self.get_failure_key_attribute(),
            AnyValue::String(type_name),
        );
    }
}

#[async_trait]
impl AdviceFilterExt for FormAuthenticationFilter {
    async fn pre_handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        ext: Option<&dyn PathMatchingFilterExt>,
    ) -> bool {
        self.authenticating_filter
            .pre_handle(request, response, ext)
            .await
    }

    fn apply_login_url(&mut self, login_url: &str) -> bool {
        // The application sets the login URL once; a filter whose login URL was
        // configured by the application itself keeps it.
        if self.get_login_url() != AccessControlFilter::<()>::DEFAULT_LOGIN_URL {
            return false;
        }

        self.set_login_url(login_url);

        true
    }

    fn apply_success_url(&mut self, success_url: &str) -> bool {
        // The application sets the page a login returns to once; a filter whose
        // success URL was configured by the application itself keeps it.
        let configured = self.authenticating_filter.get_success_url();
        if configured != AuthenticationFilter::DEFAULT_SUCCESS_URL {
            return false;
        }

        self.authenticating_filter.set_success_url(success_url);

        true
    }
}

impl Required<OncePerRequestFilter> for FormAuthenticationFilter {
    fn get_object(&self) -> &OncePerRequestFilter {
        &self.authenticating_filter.once_per_request_filter
    }

    fn get_mut_object(&mut self) -> &mut OncePerRequestFilter {
        &mut self.authenticating_filter.once_per_request_filter
    }
}

impl Required<PathMatchingFilter> for FormAuthenticationFilter {
    fn get_object(&self) -> &PathMatchingFilter {
        &self.authenticating_filter.path_matching_filter
    }

    fn get_mut_object(&mut self) -> &mut PathMatchingFilter {
        &mut self.authenticating_filter.path_matching_filter
    }
}

impl Named for FormAuthenticationFilter {
    fn name(&self) -> &str {
        "FormAuthenticationFilter"
    }
}

#[async_trait]
impl AccessControlFilterExt for FormAuthenticationFilter {
    async fn is_access_allowed(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        mapped_value: Option<Object>,
    ) -> bool {
        // A logged in subject is allowed, and the login request itself is not:
        // it is answered by the login below.
        self.authenticating_filter
            .is_access_allowed(request, response, mapped_value)
            .await
    }

    fn is_login_request(&self, request: &dyn HttpRequest, response: &dyn HttpResponse) -> bool {
        self.authenticating_filter
            .is_login_request(request, response)
    }

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
                "Attempting to access a path which requires authentication. Forwarding to the Authentication url [{}]",
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
        let username = self
            .get_username(request)
            .map(|value| value.trim().to_string())
            .unwrap_or_default();
        let password = self
            .get_password(request)
            .map(|value| value.trim().to_string())
            .unwrap_or_default();

        self.authenticating_filter
            .create_token(&username, &password, request, response, self)
    }

    fn is_remember_me(&self, request: &dyn HttpRequest) -> bool {
        // A form submission carries the flag in the body of the post, so the
        // parameter is read with the lookup that covers the query and the body.
        request
            .form_parameter(self.get_remember_me_param())
            .map(|value| WebUtils::is_true_value(value.as_ref()))
            .unwrap_or_default()
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
