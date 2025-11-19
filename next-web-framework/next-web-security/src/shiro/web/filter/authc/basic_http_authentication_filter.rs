use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use base64::{prelude::BASE64_STANDARD, Engine};
use next_web_core::{
    async_trait,
    traits::{
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
        required::Required,
    },
};
use tracing::debug;

use crate::{
    core::authc::authentication_token::AuthenticationToken,
    web::filter::{
        advice_filter::AdviceFilterExt,
        authc::{
            authenticating_filter::AuthenticatingFilterExt,
            http_authentication_filter::{HttpAuthenticationFilter, HttpAuthenticationFilterExt},
        },
        once_per_request_filter::OncePerRequestFilter,
    },
};

#[derive(Clone)]
pub struct BasicHttpAuthenticationFilter {
    http_authentication_filter: HttpAuthenticationFilter,
}

impl BasicHttpAuthenticationFilter {}

#[async_trait]
impl AdviceFilterExt for BasicHttpAuthenticationFilter {}

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
            .get_principals_and_credentials(&authz_header.unwrap(), request);

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
            .http_authentication_filter
            .authenticating_filter
            .access_control_filter
            .path_matching_filter
            .advice_filter
            .once_per_request_filter
    }

    fn get_mut_object(&mut self) -> &mut OncePerRequestFilter {
        &mut self
            .http_authentication_filter
            .authenticating_filter
            .access_control_filter
            .path_matching_filter
            .advice_filter
            .once_per_request_filter
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
