use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use base64::{Engine, prelude::BASE64_STANDARD};
use next_web_core::{
    Named, Required, async_trait,
    http::{HttpRequest, HttpResponse},
};
use tracing::debug;

use crate::{
    core::{authc::authentication_token::AuthenticationToken, util::object::Object},
    web::filter::{
        access_control_filter::AccessControlFilterExt,
        advice_filter::AdviceFilterExt,
        authc::{
            authenticating_filter::AuthenticatingFilterExt,
            http_authentication_filter::{HttpAuthenticationFilter, HttpAuthenticationFilterExt},
        },
        once_per_request_filter::OncePerRequestFilter,
        path_matching_filter::{PathMatchingFilter, PathMatchingFilterExt},
    },
};

#[derive(Clone)]
pub struct BasicHttpAuthenticationFilter {
    http_authentication_filter: HttpAuthenticationFilter,
}

impl BasicHttpAuthenticationFilter {}

#[async_trait]
impl AdviceFilterExt for BasicHttpAuthenticationFilter {
    async fn pre_handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        ext: Option<&dyn PathMatchingFilterExt>,
    ) -> bool {
        // The methods of the path this filter was configured for decide whether
        // the request is authenticated; the path matching filter holds them.
        <Self as Required<PathMatchingFilter>>::get_object(self)
            .pre_handle(request, response, ext)
            .await
    }
}

#[async_trait]
impl AccessControlFilterExt for BasicHttpAuthenticationFilter {
    async fn is_access_allowed(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        mapped_value: Option<Object>,
    ) -> bool {
        self.http_authentication_filter
            .is_access_allowed(request, response, mapped_value)
            .await
    }

    fn is_login_request(&self, request: &dyn HttpRequest, response: &dyn HttpResponse) -> bool {
        // A request that carries the credentials of the scheme is the login
        // request of an HTTP authentication filter.
        self.is_login_attempt(request, response)
    }

    async fn on_access_denied(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _mapped_value: Option<Object>,
    ) -> bool {
        let mut logged_in = false;

        if self.is_login_attempt(request, response) {
            // The token of a Basic login is the base64 pair of the header, so
            // this filter — not the HTTP filter it extends — is the one that
            // builds and authenticates it.
            if let Ok(result) = self
                .http_authentication_filter
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
impl AuthenticatingFilterExt for BasicHttpAuthenticationFilter {
    async fn create_token(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Arc<dyn AuthenticationToken> {
        let authz_header = self.get_authz_header(request);
        if authz_header.is_none() || authz_header.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
            return self
                .http_authentication_filter
                .authenticating_filter
                .create_token("", "", request, response, self);
        }
        debug!("Attempting to execute login with auth header");

        let prin_cred = self
            .http_authentication_filter
            .get_principals_and_credentials(&authz_header.unwrap(), request, self);

        if prin_cred.is_none() || prin_cred.as_ref().map(|v| v.len() < 2).unwrap_or(true) {
            let username = prin_cred
                .as_ref()
                .map(|s| s[0].as_str())
                .unwrap_or_default();
            return self
                .authenticating_filter
                .create_token(username, "", request, response, self);
        }

        let prin_cred = prin_cred.unwrap();

        let username = prin_cred.get(0).unwrap();
        let password = prin_cred.get(1).unwrap();

        self.authenticating_filter
            .create_token(username, password, request, response, self)
    }

    fn get_http_authentication_filter_ext(&self) -> Option<&dyn HttpAuthenticationFilterExt> {
        Some(self)
    }
}

impl HttpAuthenticationFilterExt for BasicHttpAuthenticationFilter {
    fn get_principals_and_credentials(&self, _scheme: &str, encoded: &str) -> Vec<String> {
        if let Ok(decoded) = BASE64_STANDARD.decode(encoded).map(String::from_utf8) {
            if let Ok(str) = decoded {
                return str.splitn(2, ':').map(ToString::to_string).collect();
            }
        }

        Vec::with_capacity(0)
    }
}

impl Required<OncePerRequestFilter> for BasicHttpAuthenticationFilter {
    fn get_object(&self) -> &OncePerRequestFilter {
        &self
            .authenticating_filter
            .path_matching_filter
            .once_per_request_filter
    }

    fn get_mut_object(&mut self) -> &mut OncePerRequestFilter {
        &mut self
            .authenticating_filter
            .path_matching_filter
            .once_per_request_filter
    }
}

impl Required<PathMatchingFilter> for BasicHttpAuthenticationFilter {
    fn get_object(&self) -> &PathMatchingFilter {
        &self.authenticating_filter.path_matching_filter
    }

    fn get_mut_object(&mut self) -> &mut PathMatchingFilter {
        &mut self.authenticating_filter.path_matching_filter
    }
}

impl Named for BasicHttpAuthenticationFilter {
    fn name(&self) -> &str {
        "BasicHttpAuthenticationFilter"
    }
}

impl Deref for BasicHttpAuthenticationFilter {
    type Target = HttpAuthenticationFilter;

    fn deref(&self) -> &Self::Target {
        &self.http_authentication_filter
    }
}

impl DerefMut for BasicHttpAuthenticationFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.http_authentication_filter
    }
}

impl Default for BasicHttpAuthenticationFilter {
    fn default() -> Self {
        let mut http_authentication_filter = HttpAuthenticationFilter::default();
        http_authentication_filter.set_authc_scheme("BASIC");
        http_authentication_filter.set_authz_scheme("BASIC");
        Self {
            http_authentication_filter,
        }
    }
}
