use std::sync::Arc;

use next_web_core::{
    async_trait,
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{
    core::{
        authc::{
            authentication_error::AuthenticationError, authentication_token::AuthenticationToken, username_password_token::UsernamePasswordToken,
        },
        subject::{self, Subject}, util::object::Object,
    },
    web::filter::{access_control_filter::AccessControlFilterExt, authc::authentication_filter::AuthenticationFilter},
};

#[derive(Clone)]
pub struct AuthenticatingFilter {
    authentication_filter: AuthenticationFilter,
}

impl AuthenticatingFilter {
    const PERMISSIVE: &str = "permissive";

    pub async fn execute_login(
        & self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authenticating_filter_ext: &dyn AuthenticatingFilterExt,
    ) -> Result<bool, BoxError> {
        let token = authenticating_filter_ext
            .create_token(request, response)
            .await;
        // let token = match authentication_token {
        //     Some(token) => token,
        //     None => return Err("create_token method implementation returned none. A valid non-null AuthenticationToken 
        //             must be created in order to execute a login attempt.".into())
        // };

        let mut subject = self
            .authentication_filter
            .access_control_filter
            .get_subject(request, response);

        if let Err(error) = subject.login(token.as_ref()) {
            return Ok(authenticating_filter_ext
                .on_login_failure(token.as_ref(), &error, request, response)
                .await);
        }

        let result = authenticating_filter_ext
            .on_login_success(token.as_ref(), subject.as_mut(), request, response)
            .await;

        match result {
            Ok(result) => Ok(result),
            Err(error) => Ok(authenticating_filter_ext
                .on_login_failure(token.as_ref(), &error, request, response)
                .await),
        }
    }

    pub fn get_host<'a>(&'a self, request: &'a dyn HttpRequest) -> Option<&'a str> {
        request.host()
    }

    pub fn create_token<S: ToString>(
        &self,
        username: S,
        password: S,
        request: &dyn HttpRequest,
        _response: &dyn HttpResponse,
        authenticating_filter_ext: &dyn AuthenticatingFilterExt,
    ) -> Arc<dyn AuthenticationToken> {
        let host = self.get_host(request);
        self.create_username_password_token(username.to_string(), password.to_string(), authenticating_filter_ext.is_remember_me(request), host)
    }

    pub fn create_username_password_token(
        &self,
        username: String,
        password: String,
        remember_me: bool,
        host: Option<&str>,
    ) -> Arc<dyn AuthenticationToken> {
        Arc::new(UsernamePasswordToken::new(username, password, remember_me, host.map(ToString::to_string)))
    }

    pub fn is_permissive(&self, mapped_value: &Object) -> bool {
        if let Some(value) = mapped_value.as_object::<Vec<String>>() {
            return value.contains(& Self::PERMISSIVE.to_string());
        }

        false
    }

    // pub fn clean_up(&self, )
}


impl AccessControlFilterExt for AuthenticatingFilter {
    fn is_access_allowed(&self, request: &dyn HttpRequest, response: &dyn HttpResponse, mapped_value: &Object) -> bool {
        self.authentication_filter.is_access_allowed(request, response, mapped_value) || (!self.is_login_request(request, response) && 
        self.is_permissive(mapped_value))
    }
}
#[async_trait]
pub trait AuthenticatingFilterExt: Send + Sync
{
    async fn create_token(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Arc<dyn AuthenticationToken>;

    async fn on_login_success(
        &self,
        token: &dyn AuthenticationToken,
        subject: &mut dyn Subject,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<bool, AuthenticationError> { Ok(true)}
 
    async fn on_login_failure(
        &self,
        token: &dyn AuthenticationToken,
        error: &AuthenticationError,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> bool { false}


    fn is_remember_me(&self, request: &dyn HttpRequest) -> bool { false }
}
