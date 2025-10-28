use std::{collections::HashSet, sync::Arc};

use axum::http::StatusCode;
use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use tracing::debug;

use crate::{
    core::{authc::authentication_token::AuthenticationToken, util::object::Object},
    web::filter::{
        access_control_filter::AccessControlFilterExt,
        authc::authenticating_filter::{AuthenticatingFilter, AuthenticatingFilterExt},
    },
};

#[derive(Clone)]
pub struct HttpAuthenticationFilter {
    authc_scheme: String,
    authz_scheme: String,
    application_name: String,

    authenticating_filter: AuthenticatingFilter,
}

impl HttpAuthenticationFilter {
    const AUTHORIZATION_HEADER: &str = "Authorization";
    const AUTHENTICATE_HEADER: &str = "WWW-Authenticate";
    const PERMISSIVE: &str = "permissive";

    pub fn get_application_name(&self) -> &str {
        &self.application_name
    }

    pub fn set_application_name(&mut self, application_name: impl ToString) {
        self.application_name = application_name.to_string();
    }

    pub fn get_authc_scheme(&self) -> &str {
        &self.authc_scheme
    }

    pub fn set_authc_scheme(&mut self, authc_scheme: impl ToString) {
        self.authc_scheme = authc_scheme.to_string();
    }

    pub fn get_authz_scheme(&self) -> &str {
        &self.authz_scheme
    }

    pub fn set_authz_scheme(&mut self, authz_scheme: impl ToString) {
        self.authz_scheme = authz_scheme.to_string();
    }

    pub fn get_authenticating_filter(&self) -> &AuthenticatingFilter {
        &self.authenticating_filter
    }

    pub fn set_authenticating_filter(&mut self, authenticating_filter: AuthenticatingFilter) {
        self.authenticating_filter = authenticating_filter;
    }

    fn http_methods_from_options(&self, options: Option<&Vec<String>>) -> Option<HashSet<String>> {
        let options = match options {
            Some(options) => {
                if options.is_empty() {
                    return None;
                }
                options
            }
            None => return None,
        };

        let mut methods = HashSet::with_capacity(options.len());

        for option in options {
            if !option.eq_ignore_ascii_case(Self::PERMISSIVE) {
                methods.insert(option.to_uppercase());
            }
        }

        None
    }

    pub fn is_login_request(&self, request: &dyn HttpRequest, response: &dyn HttpResponse) -> bool {
        self.is_login_attempt(request, response)
    }

    pub fn is_login_attempt(
        &self,
        request: &dyn HttpRequest,
        _response: &dyn HttpResponse,
    ) -> bool {
        let authz_header = self.get_authz_header(request);
        match authz_header {
            Some(h) => h
                .to_ascii_lowercase()
                .starts_with(&self.get_authz_scheme().to_ascii_lowercase()),
            None => false,
        }
    }

    pub fn get_authz_header(&self, request: &dyn HttpRequest) -> Option<String> {
        request
            .header(Self::AUTHORIZATION_HEADER)
            .map(ToString::to_string)
    }

    pub fn send_challenge(
        &self,
        _request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> bool {
        debug!("Authentication required: sending 401 Authentication challenge response.");

        response.set_status_code(StatusCode::UNAUTHORIZED);
        let authc_header = format!(
            "{} realm=\"{}\"",
            self.get_authc_scheme(),
            self.get_application_name()
        );
        response.insert_header(Self::AUTHENTICATE_HEADER.as_bytes(), &authc_header);

        false
    }

    pub fn get_principals_and_credentials(
        &self,
        authorization_header: &str,
        _request: &dyn HttpRequest,
        http_authentication_filter_ext: &dyn HttpAuthenticationFilterExt,
    ) -> Option<Vec<String>> {
        let auth_tokens = authorization_header.split(" ").collect::<Vec<&str>>();
        if auth_tokens.len() < 2 {
            return None;
        }

        Some(
            http_authentication_filter_ext
                .get_principals_and_credentials(auth_tokens[0], auth_tokens[1]),
        )
    }
}

#[async_trait]
impl AccessControlFilterExt for HttpAuthenticationFilter {
    fn is_access_allowed(
        &self,
        request: &dyn HttpRequest,
        response: &dyn HttpResponse,
        mapped_value: &Object,
    ) -> bool {
        let http_method = request.method();

        let methods = self.http_methods_from_options(mapped_value.as_object::<Vec<String>>());

        let mut authc_required = methods.as_ref().map(|x| x.len() == 0).unwrap_or_default();

        if let Some(methods) = methods.as_ref() {
            for m in methods.iter() {
                if http_method.to_string().eq(m) {
                    authc_required = true;
                    break;
                }
            }
        }

        if authc_required {
            self.authenticating_filter
                .is_access_allowed(request, response, mapped_value)
        } else {
            true
        }
    }

    async fn on_access_denied(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> bool {
        let mut logged_in = false;
        if self.is_login_attempt(request, response) {
            if let Ok(result) = self
                .authenticating_filter
                .execute_login(request, response, self)
                .await
            {
                logged_in = result;
            }
        }

        if !logged_in {
            self.send_challenge(request, response);
        }

        logged_in
    }
}

#[async_trait]
impl AuthenticatingFilterExt for HttpAuthenticationFilter {
    async fn create_token(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Arc<dyn AuthenticationToken> {
        let authz_header = self.get_authz_header(request);
        if authz_header.is_none() || authz_header.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
            return self
                .authenticating_filter
                .create_token("", "", request, response, self);
        }
        debug!("Attempting to execute login with auth header");

        let prin_cred = self.get_principals_and_credentials(&authz_header.unwrap(), request, todo!());

        if prin_cred.is_none() || prin_cred.as_ref().map(|v| v.len() < 2).unwrap_or(true) {
            let username = prin_cred.as_ref().map(|s| s[0].as_str()).unwrap_or_default();
            return self
                .authenticating_filter
                .create_token(username, "", request, response, self);
        }

        let prin_cred = prin_cred.unwrap();

        let username = prin_cred[0];
        let password = prin_cred[1];

        self.authenticating_filter
            .create_token(username, password, request, response, self)
    }
}

pub trait HttpAuthenticationFilterExt {
    fn get_principals_and_credentials(&self, scheme: &str, value: &str) -> Vec<String>;
}
