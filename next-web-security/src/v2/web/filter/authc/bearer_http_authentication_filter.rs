use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{
    Named, Required, async_trait,
    http::{HttpRequest, HttpResponse},
};
use tracing::debug;

use crate::{
    core::{
        authc::{authentication_token::AuthenticationToken, bearer_token::BearerToken},
        util::object::Object,
    },
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
pub struct BearerHttpAuthenticationFilter {
    http_authentication_filter: HttpAuthenticationFilter,
}

impl BearerHttpAuthenticationFilter {
    const BEARER: &'static str = "Bearer";

    pub fn create_bearer_token(
        &self,
        token: impl ToString,
        request: &dyn HttpRequest,
    ) -> Arc<dyn AuthenticationToken> {
        Arc::new(BearerToken::new(
            token.to_string(),
            request.host().map(ToString::to_string),
        ))
    }
}

#[async_trait]
impl AdviceFilterExt for BearerHttpAuthenticationFilter {
    async fn pre_handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        ext: Option<&dyn PathMatchingFilterExt>,
    ) -> bool {
        <Self as Required<PathMatchingFilter>>::get_object(self)
            .pre_handle(request, response, ext)
            .await
    }
}

#[async_trait]
impl AccessControlFilterExt for BearerHttpAuthenticationFilter {
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
            // The token of a Bearer login is the token of the header, so this
            // filter is the one that builds and authenticates it.
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
impl AuthenticatingFilterExt for BearerHttpAuthenticationFilter {
    async fn create_token(
        &self,
        request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
    ) -> Arc<dyn AuthenticationToken> {
        let authz_header = self.get_authz_header(request);
        if authz_header.is_none() || authz_header.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
            return self.create_bearer_token("", request);
        }
        debug!("Attempting to execute login with auth header");
        let mut prin_cred = self
            .http_authentication_filter
            .get_principals_and_credentials(&authz_header.unwrap(), request, self);

        if prin_cred.is_none() || prin_cred.as_ref().map(|v| v.is_empty()).unwrap_or(true) {
            return self.create_bearer_token("", request);
        }

        let token = prin_cred
            .as_mut()
            .map(|s| s.remove(0))
            .unwrap_or(Default::default());
        return self.create_bearer_token(token, request);
    }
}
impl HttpAuthenticationFilterExt for BearerHttpAuthenticationFilter {
    fn get_principals_and_credentials(&self, _scheme: &str, token: &str) -> Vec<String> {
        vec![token.to_string()]
    }
}

impl Required<OncePerRequestFilter> for BearerHttpAuthenticationFilter {
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

impl Required<PathMatchingFilter> for BearerHttpAuthenticationFilter {
    fn get_object(&self) -> &PathMatchingFilter {
        &self.authenticating_filter.path_matching_filter
    }

    fn get_mut_object(&mut self) -> &mut PathMatchingFilter {
        &mut self.authenticating_filter.path_matching_filter
    }
}

impl Named for BearerHttpAuthenticationFilter {
    fn name(&self) -> &str {
        "BearerHttpAuthenticationFilter"
    }
}

impl Deref for BearerHttpAuthenticationFilter {
    type Target = HttpAuthenticationFilter;

    fn deref(&self) -> &Self::Target {
        &self.http_authentication_filter
    }
}

impl DerefMut for BearerHttpAuthenticationFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.http_authentication_filter
    }
}

impl Default for BearerHttpAuthenticationFilter {
    fn default() -> Self {
        let mut http_authentication_filter = HttpAuthenticationFilter::default();
        http_authentication_filter.set_authc_scheme(Self::BEARER);
        http_authentication_filter.set_authz_scheme(Self::BEARER);
        Self {
            http_authentication_filter,
        }
    }
}
